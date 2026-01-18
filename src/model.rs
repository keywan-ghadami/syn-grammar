use syn::{Ident, Type, LitStr, Lit};
use proc_macro2::TokenStream;

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
        args: Vec<Lit>, // Für Builtins wie int_lit()
    },
    // Für Stage 0 vereinfachen wir Repeat/Optional erstmal, 
    // um den Parser einfach zu halten.
}

