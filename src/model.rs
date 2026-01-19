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
    pub pattern: Vec<ModelPattern>, // Hier nutzen wir den neuen Namen
    pub action: TokenStream,
}

#[derive(Debug, Clone)]
pub enum ModelPattern {
    Lit(LitStr),
    RuleCall {
        binding: Option<Ident>,
        rule_name: Ident,
        args: Vec<Lit>, 
    },
    Group(Vec<Vec<ModelPattern>>), 
    Bracketed(Vec<ModelPattern>),
    Braced(Vec<ModelPattern>),
    Parenthesized(Vec<ModelPattern>),
    Optional(Box<ModelPattern>),
    Repeat(Box<ModelPattern>),
    Plus(Box<ModelPattern>),
}

// Konvertierung vom Parser-Layer zum Model-Layer
impl From<crate::parser::Pattern> for ModelPattern {
    fn from(p: crate::parser::Pattern) -> Self {
        use crate::parser::Pattern as P; // Alias für bessere Lesbarkeit
        match p {
            P::Lit(l) => ModelPattern::Lit(l),
            P::RuleCall { binding, rule_name, args } => 
                ModelPattern::RuleCall { binding, rule_name, args },
            
            P::Group(alts) => ModelPattern::Group(
                alts.into_iter()
                    .map(|seq| seq.into_iter().map(ModelPattern::from).collect())
                    .collect()
            ),
            P::Bracketed(p) => ModelPattern::Bracketed(p.into_iter().map(ModelPattern::from).collect()),
            P::Braced(p) => ModelPattern::Braced(p.into_iter().map(ModelPattern::from).collect()),
            P::Parenthesized(p) => ModelPattern::Parenthesized(p.into_iter().map(ModelPattern::from).collect()),
            
            P::Optional(p) => ModelPattern::Optional(Box::new(ModelPattern::from(*p))),
            P::Repeat(p) => ModelPattern::Repeat(Box::new(ModelPattern::from(*p))),
            P::Plus(p) => ModelPattern::Plus(Box::new(ModelPattern::from(*p))),
        }
    }
}

impl ModelPattern {
    pub fn span(&self) -> Span {
        match self {
            ModelPattern::Lit(l) => l.span(),
            ModelPattern::RuleCall { rule_name, .. } => rule_name.span(),
            ModelPattern::Optional(p) | ModelPattern::Repeat(p) | ModelPattern::Plus(p) => p.span(),
            // Für Gruppen und Blöcke nutzen wir den Call-Site Span als Fallback, 
            // es sei denn, man speichert die Token-Spans im Parser.
            _ => Span::call_site(),
        }
    }
}
