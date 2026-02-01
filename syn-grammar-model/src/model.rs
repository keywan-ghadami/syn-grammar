// Moved from macros/src/model.rs
use crate::parser;
use proc_macro2::{Span, TokenStream};
use syn::{Attribute, Ident, Lit, LitStr, Type};

#[derive(Debug, Clone)]
pub struct GrammarDefinition {
    pub name: Ident,
    pub inherits: Option<Ident>,
    pub rules: Vec<Rule>,
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub attrs: Vec<Attribute>,
    pub is_pub: bool,
    pub name: Ident,
    pub params: Vec<(Ident, Type)>,
    pub return_type: Type,
    pub variants: Vec<RuleVariant>,
}

#[derive(Debug, Clone)]
pub struct RuleVariant {
    pub pattern: Vec<ModelPattern>,
    pub action: TokenStream,
}

#[derive(Debug, Clone)]
pub enum ModelPattern {
    Cut,
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
    Recover {
        binding: Option<Ident>,
        body: Box<ModelPattern>,
        sync: Box<ModelPattern>,
    },
}

impl From<parser::GrammarDefinition> for GrammarDefinition {
    fn from(p: parser::GrammarDefinition) -> Self {
        Self {
            name: p.name,
            inherits: p.inherits.map(|spec| spec.name),
            rules: p.rules.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<parser::Rule> for Rule {
    fn from(p: parser::Rule) -> Self {
        Self {
            attrs: p.attrs,
            is_pub: p.is_pub.is_some(),
            name: p.name,
            params: p
                .params
                .into_iter()
                .map(|param| (param.name, param.ty))
                .collect(),
            return_type: p.return_type,
            variants: p.variants.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<parser::RuleVariant> for RuleVariant {
    fn from(p: parser::RuleVariant) -> Self {
        Self {
            pattern: p.pattern.into_iter().map(Into::into).collect(),
            action: p.action,
        }
    }
}

impl From<parser::Pattern> for ModelPattern {
    fn from(p: parser::Pattern) -> Self {
        use parser::Pattern as P;
        match p {
            P::Cut => ModelPattern::Cut,
            P::Lit(l) => ModelPattern::Lit(l),
            P::RuleCall {
                binding,
                rule_name,
                args,
            } => ModelPattern::RuleCall {
                binding,
                rule_name,
                args,
            },
            P::Group(alts) => ModelPattern::Group(
                alts.into_iter()
                    .map(|seq| seq.into_iter().map(ModelPattern::from).collect())
                    .collect(),
            ),
            P::Bracketed(p) => {
                ModelPattern::Bracketed(p.into_iter().map(ModelPattern::from).collect())
            }
            P::Braced(p) => ModelPattern::Braced(p.into_iter().map(ModelPattern::from).collect()),
            P::Parenthesized(p) => {
                ModelPattern::Parenthesized(p.into_iter().map(ModelPattern::from).collect())
            }
            P::Optional(p) => ModelPattern::Optional(Box::new(ModelPattern::from(*p))),
            P::Repeat(p) => ModelPattern::Repeat(Box::new(ModelPattern::from(*p))),
            P::Plus(p) => ModelPattern::Plus(Box::new(ModelPattern::from(*p))),
            P::Recover {
                binding,
                body,
                sync,
            } => ModelPattern::Recover {
                binding,
                body: Box::new(ModelPattern::from(*body)),
                sync: Box::new(ModelPattern::from(*sync)),
            },
        }
    }
}

impl ModelPattern {
    pub fn span(&self) -> Span {
        match self {
            ModelPattern::Cut => Span::call_site(),
            ModelPattern::Lit(l) => l.span(),
            ModelPattern::RuleCall { rule_name, .. } => rule_name.span(),
            ModelPattern::Optional(p) | ModelPattern::Repeat(p) | ModelPattern::Plus(p) => p.span(),
            ModelPattern::Recover { body, .. } => body.span(),
            ModelPattern::Group(alts) => alts
                .first()
                .and_then(|seq| seq.first())
                .map(|p| p.span())
                .unwrap_or_else(Span::call_site),
            ModelPattern::Bracketed(content)
            | ModelPattern::Braced(content)
            | ModelPattern::Parenthesized(content) => content
                .first()
                .map(|p| p.span())
                .unwrap_or_else(Span::call_site),
        }
    }
}
