use syn::{Ident, Type, LitStr, Lit};
use proc_macro2::{TokenStream, Span};

#[derive(Debug, Clone)]
pub struct GrammarDefinition {
    pub name: Ident,
    pub inherits: Option<Ident>, 
    pub rules: Vec<Rule>,
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub is_pub: bool,
    pub name: Ident,
    pub return_type: Type,
    pub variants: Vec<RuleVariant>,
}

#[derive(Debug, Clone)]
pub struct RuleVariant {
    pub pattern: Vec<Pattern>,
    pub action: TokenStream,
}

#[derive(Debug, Clone)]
pub enum Pattern {
    Lit(LitStr),
    RuleCall {
        binding: Option<Ident>,
        rule_name: Ident,
        args: Vec<Lit>, 
    },
    Group(Vec<Vec<Pattern>>), // ( A | B )
    Optional(Box<Pattern>),   // A?
    Repeat(Box<Pattern>),     // A*
    Plus(Box<Pattern>),       // A+
}

impl Pattern {
    pub fn span(&self) -> Span {
        match self {
            Pattern::Lit(l) => syn::spanned::Spanned::span(l),
            Pattern::RuleCall { rule_name, .. } => syn::spanned::Spanned::span(rule_name),
            Pattern::Group(alts) => {
                alts.first()
                    .and_then(|seq| seq.first())
                    .map(|p| p.span())
                    .unwrap_or_else(Span::call_site)
            },
            Pattern::Optional(p) => p.span(),
            Pattern::Repeat(p) => p.span(),
            Pattern::Plus(p) => p.span(),
        }
    }
}
