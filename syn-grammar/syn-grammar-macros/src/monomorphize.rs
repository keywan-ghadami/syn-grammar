use crate::backend::SynBackend;
use proc_macro2::Span;
use quote::format_ident;
use std::collections::{hash_map::DefaultHasher, HashMap};
use std::hash::{Hash, Hasher};
use syn::spanned::Spanned;
use syn::visit_mut::VisitMut;
use syn::{parse_quote, Ident, Path, Type};
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
            let has_untyped_params = rule.params.iter().any(|p| p.ty.is_none());
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
                rule_path,
                args,
                generics,
                ..
            } => {
                for arg in args.iter_mut() {
                    match arg {
                        Argument::Positional(p) | Argument::Named(_, p) => {
                            self.expand_pattern(p);
                        }
                    }
                }

                let flattened_name = flatten_path(rule_path);
                if let Some(template) = self.templates.get(&flattened_name).cloned() {
                    let new_name = self.instantiate(&template, args, generics);
                    *rule_path = Path::from(new_name);
                    args.clear();
                    generics.clear();
                }
            }
            ModelPattern::Group { alts, .. } => {
                for (seq, _, _) in alts {
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

    fn instantiate(&mut self, template: &Rule, args: &[Argument], generics: &[Type]) -> Ident {
        // Extract ModelPatterns from Arguments
        let model_patterns: Vec<&ModelPattern> = args
            .iter()
            .map(|a| match a {
                Argument::Positional(p) => p,
                Argument::Named(_, p) => p,
            })
            .collect();

        // Include generics in the key/hash for instantiation
        let args_repr = model_patterns
            .iter()
            .map(|a| format!("{:?}", a))
            .collect::<Vec<_>>()
            .join(",");

        let generics_repr = generics
            .iter()
            .map(|t| quote::quote!(#t).to_string())
            .collect::<Vec<_>>()
            .join(",");

        let unique_key = format!("{}<{}>({})", template.name, generics_repr, args_repr);
        let key = (template.name.clone(), unique_key.clone());

        if let Some(name) = self.instantiations.get(&key) {
            return name.clone();
        }

        let mut hasher = DefaultHasher::new();
        unique_key.hash(&mut hasher);
        let hash = hasher.finish();
        let new_name = format_ident!("{}_{:x}", template.name, hash);

        self.instantiations.insert(key, new_name.clone());

        let mut grammar_params = Vec::new();
        for p in &template.params {
            if p.ty.is_none() {
                grammar_params.push(p.name.clone());
            }
        }

        let param_map: HashMap<Ident, ModelPattern> = grammar_params
            .iter()
            .zip(model_patterns.iter())
            .map(|(k, v)| (k.clone(), (*v).clone()))
            .collect();

        let mut new_rule = template.clone();
        new_rule.name = new_name.clone();
        let old_generics = new_rule.generics.clone();
        new_rule.generics.params.clear();

        new_rule.params.retain(|p| p.ty.is_some());

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

        // Map provided generics to template generic params
        if generic_params.len() <= generics.len() {
            for (i, gp) in generic_params.iter().enumerate() {
                type_map.insert(gp.clone(), generics[i].clone());
            }
        } else {
            // Try to infer from args if explicit generics are missing
            if generic_params.len() <= model_patterns.len() {
                for (i, gp) in generic_params.iter().enumerate() {
                    if let Some(ty) = self.infer_type(model_patterns[i]) {
                        type_map.insert(gp.clone(), ty);
                    }
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

        // Substitute types in patterns (generics in rule calls)
        for variant in &mut new_rule.variants {
            for pattern in &mut variant.pattern {
                substitute_types_in_pattern(pattern, &mut type_substituter);
            }
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
            ModelPattern::Lit { .. } => Some(parse_quote!(())),
            ModelPattern::RuleCall { rule_path, .. } => {
                let name = flatten_path(rule_path);
                self.rule_types.get(&name).cloned()
            }
            _ => None,
        }
    }
}

fn flatten_path(path: &Path) -> Ident {
    let segments: Vec<_> = path.segments.iter().collect();

    let s = segments
        .iter()
        .map(|s| s.ident.to_string())
        .collect::<Vec<_>>()
        .join("__");
    Ident::new(&s, path.span())
}

fn substitute_types_in_pattern(pattern: &mut ModelPattern, substituter: &mut TypeSubstituter) {
    match pattern {
        ModelPattern::RuleCall { generics, args, .. } => {
            for ty in generics {
                substituter.visit_type_mut(ty);
            }
            for arg in args {
                match arg {
                    Argument::Positional(p) | Argument::Named(_, p) => {
                        substitute_types_in_pattern(p, substituter);
                    }
                }
            }
        }
        ModelPattern::Group { alts, .. } => {
            for (seq, _, _) in alts {
                for p in seq {
                    substitute_types_in_pattern(p, substituter);
                }
            }
        }
        ModelPattern::Bracketed(p, _)
        | ModelPattern::Braced(p, _)
        | ModelPattern::Parenthesized(p, _) => {
            for sub in p {
                substitute_types_in_pattern(sub, substituter);
            }
        }
        ModelPattern::Optional(p, _)
        | ModelPattern::Repeat(p, _)
        | ModelPattern::Plus(p, _)
        | ModelPattern::SpanBinding(p, _, _)
        | ModelPattern::Peek(p, _)
        | ModelPattern::Not(p, _) => {
            substitute_types_in_pattern(p, substituter);
        }
        ModelPattern::Recover { body, sync, .. } => {
            substitute_types_in_pattern(body, substituter);
            substitute_types_in_pattern(sync, substituter);
        }
        ModelPattern::Until { pattern, .. } => {
            substitute_types_in_pattern(pattern, substituter);
        }
        _ => {}
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
                rule_path,
                args,
                ..
            } => {
                let old_binding = binding.clone();
                // Check if rule_path matches a parameter
                let is_match = if let Some(ident) = rule_path.get_ident() {
                    self.param_map.contains_key(ident)
                } else {
                    false
                };

                if is_match {
                    let ident = rule_path.get_ident().unwrap();
                    let replacement = self.param_map.get(ident).unwrap();
                    *pattern = replacement.clone();

                    if let Some(b) = old_binding {
                        // The binding of the call site is transferred to the substituted
                        // pattern - but only if that pattern does not already bring one
                        // of its own. Both variants behave the same, hence a shared
                        // arm with a guard condition.
                        match pattern {
                            ModelPattern::RuleCall { binding: new_b, .. }
                            | ModelPattern::Recover { binding: new_b, .. }
                                if new_b.is_none() =>
                            {
                                *new_b = Some(b);
                            }
                            _ => {}
                        }
                    }
                } else {
                    for arg in args {
                        match arg {
                            Argument::Positional(p) | Argument::Named(_, p) => {
                                self.visit_pattern(p);
                            }
                        }
                    }
                }
            }
            ModelPattern::Group { alts, .. } => {
                for (seq, _, _) in alts {
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
            let segments: Vec<_> = tp.path.segments.iter().collect();
            if tp.qself.is_none() && segments.len() == 1 {
                let ident = &segments[0].ident;
                if let Some(replacement) = self.type_map.get(ident) {
                    *i = replacement.clone();
                    return;
                }
            }
        }
        syn::visit_mut::visit_type_mut(self, i);
    }
}
