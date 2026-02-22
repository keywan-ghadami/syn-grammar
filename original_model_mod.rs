use crate::parser;
use proc_macro2::{Ident, TokenStream};

pub mod backend;
pub mod types;

#[derive(Debug, Clone)]
pub struct Grammar {
    pub name: Ident,
    pub rules: Vec<Rule>,
    pub uses: Vec<syn::ItemUse>,
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub name: Ident,
    pub generics: syn::Generics,
    pub params: Vec<RuleParameter>,
    pub return_type: syn::Type,
    pub variants: Vec<RuleVariant>,
    pub is_pub: bool,
    pub attrs: Vec<syn::Attribute>,
}

#[derive(Debug, Clone)]
pub struct RuleParameter {
    pub name: Ident,
    pub ty: syn::Type,
}

#[derive(Debug, Clone)]
pub struct RuleVariant {
    pub pattern: Vec<Pattern>,
    pub action: TokenStream,
    pub label: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Pattern {
    Cut,
    Lit {
        binding: Option<Ident>,
        lit: syn::Lit,
    },
    RuleCall {
        binding: Option<Ident>,
        rule_name: Ident,
        generics: Vec<syn::Type>,
        args: Vec<Argument>,
    },
    Group(Vec<(Vec<Pattern>, Option<TokenStream>, Option<String>)>),
    Bracketed(Vec<Pattern>),
    Braced(Vec<Pattern>),
    Parenthesized(Vec<Pattern>),
    Optional(Box<Pattern>),
    Repeat(Box<Pattern>),
    Plus(Box<Pattern>),
    SpanBinding(Box<Pattern>, Ident),
    Recover {
        binding: Option<Ident>,
        body: Box<Pattern>,
        sync: Box<Pattern>,
    },
    Peek(Box<Pattern>),
    Not(Box<Pattern>),
    Until {
        binding: Option<Ident>,
        pattern: Box<Pattern>,
    },
}

#[derive(Debug, Clone)]
pub enum Argument {
    Positional(Pattern),
    Named(Ident, Pattern),
}

pub fn morphism<F: Morphism>(p: parser::GrammarDefinition) -> Grammar {
    Grammar {
        name: p.name,
        rules: p.rules.into_iter().map(F::rule).collect(),
        uses: p.uses,
    }
}

pub trait Morphism {
    fn rule(p: parser::Rule) -> Rule;
    fn rule_parameter(p: parser::RuleParameter) -> RuleParameter;
    fn rule_variant(p: parser::RuleVariant) -> RuleVariant;
    fn pattern(p: parser::Pattern) -> Pattern;
    fn argument(p: parser::Argument) -> Argument;
}

impl<F: Morphism> Morphism for &F {
    fn rule(p: parser::Rule) -> Rule {
        F::rule(p)
    }

    fn rule_parameter(p: parser::RuleParameter) -> RuleParameter {
        F::rule_parameter(p)
    }

    fn rule_variant(p: parser::RuleVariant) -> RuleVariant {
        F::rule_variant(p)
    }

    fn pattern(p: parser::Pattern) -> Pattern {
        F::pattern(p)
    }

    fn argument(p: parser::Argument) -> Argument {
        F::argument(p)
    }
}

pub struct IdentityMorphism;

impl Morphism for IdentityMorphism {
    fn rule(p: parser::Rule) -> Rule {
        Rule {
            name: p.name,
            generics: p.generics,
            params: p
                .params
                .into_iter()
                .map(IdentityMorphism::rule_parameter)
                .collect(),
            return_type: p.return_type,
            variants: p
                .variants
                .into_iter()
                .map(IdentityMorphism::rule_variant)
                .collect(),
            is_pub: p.is_pub.is_some(),
            attrs: p.attrs,
        }
    }

    fn rule_parameter(p: parser::RuleParameter) -> RuleParameter {
        RuleParameter {
            name: p.name,
            ty: p.ty.unwrap(),
        }
    }

    fn rule_variant(p: parser::RuleVariant) -> RuleVariant {
        RuleVariant {
            pattern: p.pattern.into_iter().map(IdentityMorphism::pattern).collect(),
            action: p.action,
            label: p.label,
        }
    }

    fn pattern(p: parser::Pattern) -> Pattern {
        match p {
            parser::Pattern::Cut(_) => Pattern::Cut,
            parser::Pattern::Lit { binding, lit } => Pattern::Lit { binding, lit },
            parser::Pattern::RuleCall {
                binding,
                rule_path,
                generics,
                args,
            } => Pattern::RuleCall {
                binding,
                rule_name: rule_path.segments.last().unwrap().ident.clone(),
                generics,
                args: args.into_iter().map(IdentityMorphism::argument).collect(),
            },
            parser::Pattern::Group(alts, _) => Pattern::Group(
                alts.into_iter()
                    .map(|(seq, action, label)| {
                        (
                            seq.into_iter().map(IdentityMorphism::pattern).collect(),
                            action,
                            label,
                        )
                    })
                    .collect(),
            ),
            parser::Pattern::Bracketed(seq, _) => Pattern::Bracketed(
                seq.into_iter().map(IdentityMorphism::pattern).collect(),
            ),
            parser::Pattern::Braced(seq, _) => {
                Pattern::Braced(seq.into_iter().map(IdentityMorphism::pattern).collect())
            }
            parser::Pattern::Parenthesized(seq, _, _) => Pattern::Parenthesized(
                seq.into_iter().map(IdentityMorphism::pattern).collect(),
            ),
            parser::Pattern::Optional(p, _) => {
                Pattern::Optional(Box::new(IdentityMorphism::pattern(*p)))
            }
            parser::Pattern::Repeat(p, _) => Pattern::Repeat(Box::new(IdentityMorphism::pattern(*p))),
            parser::Pattern::Plus(p, _) => Pattern::Plus(Box::new(IdentityMorphism::pattern(*p))),
            parser::Pattern::SpanBinding(p, id, _) => {
                Pattern::SpanBinding(Box::new(IdentityMorphism::pattern(*p)), id)
            }
            parser::Pattern::Recover {
                binding,
                body,
                sync,
                ..
            } => Pattern::Recover {
                binding,
                body: Box::new(IdentityMorphism::pattern(*body)),
                sync: Box::new(IdentityMorphism::pattern(*sync)),
            },
            parser::Pattern::Peek(p, _) => Pattern::Peek(Box::new(IdentityMorphism::pattern(*p))),
            parser::Pattern::Not(p, _) => Pattern::Not(Box::new(IdentityMorphism::pattern(*p))),
            parser::Pattern::Until {
                binding, pattern, ..
            } => Pattern::Until {
                binding,
                pattern: Box::new(IdentityMorphism::pattern(*pattern)),
            },
        }
    }

    fn argument(p: parser::Argument) -> Argument {
        match p {
            parser::Argument::Positional(p) => {
                Argument::Positional(IdentityMorphism::pattern(p))
            }
            parser::Argument::Named(n, p) => Argument::Named(n, IdentityMorphism::pattern(p)),
        }
    }
}
