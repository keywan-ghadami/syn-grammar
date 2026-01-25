use crate::parser; // Zugriff auf die Syntax-Strukturen
use syn::{Ident, Type, LitStr, Lit};
use proc_macro2::{TokenStream, Span};

// --- Datenstrukturen (Das saubere "Semantic Model") ---

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
}

// --- Konvertierungs-Logik (Parser -> Model) ---

impl From<parser::GrammarDefinition> for GrammarDefinition {
    fn from(p: parser::GrammarDefinition) -> Self {
        Self {
            name: p.name,
            // Extrahiere nur den Namen aus der InheritanceSpec
            inherits: p.inherits.map(|spec| spec.name),
            rules: p.rules.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<parser::Rule> for Rule {
    fn from(p: parser::Rule) -> Self {
        Self {
            // Option<Token![pub]> -> bool
            is_pub: p.is_pub.is_some(),
            name: p.name,
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
            P::RuleCall { binding, rule_name, args } => 
                ModelPattern::RuleCall { binding, rule_name, args },
            
            // Rekursive Konvertierung für verschachtelte Strukturen
            P::Group(alts) => ModelPattern::Group(
                alts.into_iter()
                    .map(|seq| seq.into_iter().map(ModelPattern::from).collect())
                    .collect()
            ),
            P::Bracketed(p) => ModelPattern::Bracketed(p.into_iter().map(ModelPattern::from).collect()),
            P::Braced(p) => ModelPattern::Braced(p.into_iter().map(ModelPattern::from).collect()),
            P::Parenthesized(p) => ModelPattern::Parenthesized(p.into_iter().map(ModelPattern::from).collect()),
            
            // Rekursion für Boxed Types
            P::Optional(p) => ModelPattern::Optional(Box::new(ModelPattern::from(*p))),
            P::Repeat(p) => ModelPattern::Repeat(Box::new(ModelPattern::from(*p))),
            P::Plus(p) => ModelPattern::Plus(Box::new(ModelPattern::from(*p))),
        }
    }
}

// --- Hilfsmethoden ---

impl ModelPattern {
    /// Liefert den Span des Patterns für genaue Fehlermeldungen im Generator via quote_spanned!
    pub fn span(&self) -> Span {
        match self {
            ModelPattern::Cut => Span::call_site(),
            ModelPattern::Lit(l) => l.span(),
            ModelPattern::RuleCall { rule_name, .. } => rule_name.span(),
            ModelPattern::Optional(p) | ModelPattern::Repeat(p) | ModelPattern::Plus(p) => p.span(),
            // Für Gruppen nehmen wir den Span des ersten Elements oder call_site als Fallback
            ModelPattern::Group(alts) => alts.first().and_then(|seq| seq.first()).map(|p| p.span()).unwrap_or_else(Span::call_site),
            // Bei Klammern wäre es ideal, den Span der Klammern selbst zu haben, 
            // aber da wir den hier nicht speichern, nehmen wir den Inhalt oder call_site.
            ModelPattern::Bracketed(content) | 
            ModelPattern::Braced(content) | 
            ModelPattern::Parenthesized(content) => content.first().map(|p| p.span()).unwrap_or_else(Span::call_site),
        }
    }
}
