use syn::{Ident, Type, LitStr, Lit};
use proc_macro2::{TokenStream, Span};
use syn_grammar_macros::ModelConvert;

#[derive(Debug, Clone, ModelConvert)]
pub struct GrammarDefinition {
    pub name: Ident,
    pub inherits: Option<Ident>, 
    pub rules: Vec<Rule>,
}

#[derive(Debug, Clone, ModelConvert)]
pub struct Rule {
    pub is_pub: bool, 
    pub name: Ident,
    pub return_type: Type,
    pub variants: Vec<RuleVariant>,
}

#[derive(Debug, Clone, ModelConvert)]
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
    Group(Vec<Vec<Pattern>>), 
    Bracketed(Vec<Pattern>),
    Braced(Vec<Pattern>),
    Parenthesized(Vec<Pattern>),
    Optional(Box<Pattern>),
    Repeat(Box<Pattern>),
    Plus(Box<Pattern>),
}

// Da ModelConvert nur auf Structs arbeitet, mappen wir das Enum manuell,
// was aber durch .into() Aufrufe in den Struct-Mappings unterstützt wird.
impl From<crate::parser::Pattern> for Pattern {
    fn from(p: crate::parser::Pattern) -> Self {
        match p {
            crate::parser::Pattern::Lit(l) => Pattern::Lit(l),
            crate::parser::Pattern::RuleCall { binding, rule_name, args } => 
                Pattern::RuleCall { binding, rule_name, args },
            crate::parser::Pattern::Group(alts) => 
                Pattern::Group(alts.into_iter().map(|seq| seq.into_iter().map(Into::into).collect()).collect()),
            crate::parser::Pattern::Bracketed(p) => Pattern::Bracketed(p.into_iter().map(Into::into).collect()),
            crate::parser::Pattern::Braced(p) => Pattern::Braced(p.into_iter().map(Into::into).collect()),
            crate::parser::Pattern::Parenthesized(p) => Pattern::Parenthesized(p.into_iter().map(Into::into).collect()),
            crate::parser::Pattern::Optional(p) => Pattern::Optional(Box::new((*p).into())),
            crate::parser::Pattern::Repeat(p) => Pattern::Repeat(Box::new((*p).into())),
            crate::parser::Pattern::Plus(p) => Pattern::Plus(Box::new((*p).into())),
        }
    }
}

impl Pattern {
    pub fn span(&self) -> Span {
        match self {
            Pattern::Lit(l) => syn::spanned::Spanned::span(l),
            Pattern::RuleCall { rule_name, .. } => syn::spanned::Spanned::span(rule_name),
            Pattern::Optional(p) | Pattern::Repeat(p) | Pattern::Plus(p) => p.span(),
            _ => Span::call_site(),
        }
    }
}
