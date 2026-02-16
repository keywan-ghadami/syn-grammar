use crate::backend::SynBackend;
use proc_macro2::Span;
use quote::format_ident;
use std::collections::{hash_map::DefaultHasher, HashMap};
use std::hash::{Hash, Hasher};
use syn::visit_mut::VisitMut;
use syn::{parse_quote, Ident, Type};
use syn_grammar_model::model::*;
use syn_grammar_model::Backend;

pub struct Monomorphizer {
    templates: HashMap<Ident, Rule>,
    instantiations: HashMap<(Ident, String), Ident>,
    processed_rules: Vec<Rule>,
    pending_rules: Vec<Rule>,
    rule_types: HashMap<Ident, Type>,
}

impl Monomorphizer {
    pub fn new(rules: Vec<Rule>) -> Self {
        let mut templates = HashMap::new();
        let mut rule_types = HashMap::new();
        let mut pending_rules = Vec::new();

        // Load builtins from backend
        for b in SynBackend::get_builtins() {
            if let Ok(ty) = syn::parse_str::<Type>(b.return_type) {
                rule_types.insert(Ident::new(b.name, Span::call_site()), ty);
            }
        }

        for rule in rules {
            let has_generics = !rule.generics.params.is_empty();
            let has_untyped_params = rule.params.iter().any(|(_, ty)| ty.is_none());
            let is_generic = has_generics || has_untyped_params;

            if is_generic {
                templates.insert(rule.name.clone(), rule);
            } else {
                rule_types.insert(rule.name.clone(), rule.return_type.clone());
                pending_rules.push(rule);
            }
        }

        Self {
            templates,
            instantiations: HashMap::new(),
            processed_rules: Vec::new(),
            pending_rules,
            rule_types,
        }
    }

    pub fn process(mut self) -> Vec<Rule> {
        while let Some(mut rule) = self.pending_rules.pop() {
            self.expand_rule(&mut rule);
            self.processed_rules.push(rule);
        }
        self.processed_rules
    }

    fn expand_rule(&mut self, rule: &mut Rule) {
        for variant in &mut rule.variants {
            for pattern in &mut variant.pattern {
                self.expand_pattern(pattern);
            }
        }
    }

    fn expand_pattern(&mut self, pattern: &mut ModelPattern) {
        match pattern {
            ModelPattern::RuleCall {
                rule_name, args, ..
            } => {
                for arg in args.iter_mut() {
                    self.expand_pattern(arg);
                }

                if let Some(template) = self.templates.get(rule_name).cloned() {
                    let new_name = self.instantiate(&template, args);
                    *rule_name = new_name;
                    args.clear();
                }
            }
            ModelPattern::Group(alts, _) => {
                for seq in alts {
                    for p in seq {
                        self.expand_pattern(p);
                    }
                }
            }
            ModelPattern::Bracketed(p, _)
            | ModelPattern::Braced(p, _)
            | ModelPattern::Parenthesized(p, _) => {
                for sub in p {
                    self.expand_pattern(sub);
                }
            }
            ModelPattern::Optional(p, _)
            | ModelPattern::Repeat(p, _)
            | ModelPattern::Plus(p, _)
            | ModelPattern::SpanBinding(p, _, _)
            | ModelPattern::Peek(p, _)
            | ModelPattern::Not(p, _) => {
                self.expand_pattern(p);
            }
            ModelPattern::Recover { body, sync, .. } => {
                self.expand_pattern(body);
                self.expand_pattern(sync);
            }
            _ => {}
        }
    }

    fn instantiate(&mut self, template: &Rule, args: &[ModelPattern]) -> Ident {
        let args_repr = args
            .iter()
            .map(|a| format!("{:?}", a))
            .collect::<Vec<_>>()
            .join(",");
        let key = (template.name.clone(), args_repr.clone());

        if let Some(name) = self.instantiations.get(&key) {
            return name.clone();
        }

        let mut hasher = DefaultHasher::new();
        args_repr.hash(&mut hasher);
        let hash = hasher.finish();
        let new_name = format_ident!("{}_{:x}", template.name, hash);

        self.instantiations.insert(key, new_name.clone());

        let mut grammar_params = Vec::new();
        for (name, ty) in &template.params {
            if ty.is_none() {
                grammar_params.push(name.clone());
            }
        }

        let param_map: HashMap<Ident, ModelPattern> = grammar_params
            .iter()
            .zip(args.iter())
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        let mut new_rule = template.clone();
        new_rule.name = new_name.clone();
        let old_generics = new_rule.generics.clone();
        new_rule.generics.params.clear();

        new_rule.params.retain(|(_, ty)| ty.is_some());

        let substituter = ParamSubstituter {
            param_map: &param_map,
        };
        for variant in &mut new_rule.variants {
            for pattern in &mut variant.pattern {
                substituter.visit_pattern(pattern);
            }
        }

        let mut type_map = HashMap::new();
        let generic_params: Vec<Ident> = old_generics
            .type_params()
            .map(|tp| tp.ident.clone())
            .collect();

        if generic_params.len() <= args.len() {
            for (i, gp) in generic_params.iter().enumerate() {
                let arg = &args[i];
                if let Some(ty) = self.infer_type(arg) {
                    type_map.insert(gp.clone(), ty);
                }
            }
        }

        let mut type_substituter = TypeSubstituter {
            type_map: &type_map,
        };

        type_substituter.visit_type_mut(&mut new_rule.return_type);

        if let Some(where_clause) = &mut new_rule.generics.where_clause {
            type_substituter.visit_where_clause_mut(where_clause);
        }

        for variant in &mut new_rule.variants {
            if let Ok(mut block) = syn::parse2::<syn::Block>(variant.action.clone()) {
                type_substituter.visit_block_mut(&mut block);
                variant.action = quote::quote!(#block);
            }
        }

        self.rule_types
            .insert(new_name.clone(), new_rule.return_type.clone());
        self.pending_rules.push(new_rule);

        new_name
    }

    fn infer_type(&self, pattern: &ModelPattern) -> Option<Type> {
        match pattern {
            ModelPattern::Lit(_) => Some(parse_quote!(())),
            ModelPattern::RuleCall { rule_name, .. } => self.rule_types.get(rule_name).cloned(),
            _ => None,
        }
    }
}

struct ParamSubstituter<'a> {
    param_map: &'a HashMap<Ident, ModelPattern>,
}

impl<'a> ParamSubstituter<'a> {
    fn visit_pattern(&self, pattern: &mut ModelPattern) {
        match pattern {
            ModelPattern::RuleCall {
                binding,
                rule_name,
                args,
                ..
            } => {
                let old_binding = binding.clone();
                if let Some(replacement) = self.param_map.get(rule_name) {
                    *pattern = replacement.clone();

                    if let Some(b) = old_binding {
                        match pattern {
                            ModelPattern::RuleCall {
                                binding: ref mut new_b,
                                ..
                            } => {
                                if new_b.is_none() {
                                    *new_b = Some(b);
                                }
                            }
                            ModelPattern::Recover {
                                binding: ref mut new_b,
                                ..
                            } => {
                                if new_b.is_none() {
                                    *new_b = Some(b);
                                }
                            }
                            _ => {}
                        }
                    }
                } else {
                    for arg in args {
                        self.visit_pattern(arg);
                    }
                }
            }
            ModelPattern::Group(alts, _) => {
                for seq in alts {
                    for p in seq {
                        self.visit_pattern(p);
                    }
                }
            }
            ModelPattern::Bracketed(p, _)
            | ModelPattern::Braced(p, _)
            | ModelPattern::Parenthesized(p, _) => {
                for sub in p {
                    self.visit_pattern(sub);
                }
            }
            ModelPattern::Optional(p, _)
            | ModelPattern::Repeat(p, _)
            | ModelPattern::Plus(p, _)
            | ModelPattern::SpanBinding(p, _, _)
            | ModelPattern::Peek(p, _)
            | ModelPattern::Not(p, _) => {
                self.visit_pattern(p);
            }
            ModelPattern::Recover { body, sync, .. } => {
                self.visit_pattern(body);
                self.visit_pattern(sync);
            }
            _ => {}
        }
    }
}

struct TypeSubstituter<'a> {
    type_map: &'a HashMap<Ident, Type>,
}

impl<'a> VisitMut for TypeSubstituter<'a> {
    fn visit_type_mut(&mut self, i: &mut Type) {
        if let Type::Path(tp) = i {
            if tp.qself.is_none() && tp.path.segments.len() == 1 {
                let ident = &tp.path.segments[0].ident;
                if let Some(replacement) = self.type_map.get(ident) {
                    *i = replacement.clone();
                    return;
                }
            }
        }
        syn::visit_mut::visit_type_mut(self, i);
    }
}
