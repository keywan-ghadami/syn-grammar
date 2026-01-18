use crate::model::*;
use syn::parse::{Parse, ParseStream};
use syn::{Result, Token, token, LitStr, Ident};
use derive_syn_parse::Parse;

mod kw {
    syn::custom_keyword!(grammar);
    syn::custom_keyword!(rule);
    syn::custom_keyword!(paren);
}

// 1. Grammar Definition
#[derive(Parse)]
pub struct GrammarDefinition {
    #[syn(parenthesized)]
    pub _paren: Option<token::Paren>, // Optional wrapper
    pub _kw_gram: kw::grammar,
    pub name: Ident,
    #[syn(peek = Token![:])]
    pub inherits: Option<InheritanceSpec>, // Helper struct für optionale Vererbung
    #[syn(brace)]
    pub _brace: token::Brace,
    #[syn(in = _brace)]
    #[parse(ParseRule::parse_list)] // Eigene Helper Funktion für Liste von Regeln
    pub rules: Vec<Rule>,
}

#[derive(Parse)]
pub struct InheritanceSpec {
    pub _colon: Token![:],
    pub name: Ident,
}

// Helper für Vec<Rule>
struct ParseRule;
impl ParseRule {
    fn parse_list(input: ParseStream) -> Result<Vec<Rule>> {
        let mut rules = Vec::new();
        while !input.is_empty() {
            rules.push(input.parse()?);
        }
        Ok(rules)
    }
}

// 2. Rule
#[derive(Parse)]
pub struct Rule {
    #[syn(peek = Token![pub])]
    pub is_pub: Option<Token![pub]>,
    pub _kw_rule: kw::rule,
    pub name: Ident,
    pub _arrow: Token![->],
    pub return_type: syn::Type,
    pub _eq: Token![=],
    #[parse(ParseVariant::parse_list)]
    pub variants: Vec<RuleVariant>,
}

struct ParseVariant;
impl ParseVariant {
    fn parse_list(input: ParseStream) -> Result<Vec<RuleVariant>> {
        let mut variants = Vec::new();
        loop {
            // Pattern Parsen bis -> oder |
            let mut pattern = Vec::new();
            while !input.peek(Token![->]) && !input.peek(Token![|]) && !input.is_empty() {
                pattern.push(input.parse()?);
            }

            let _arrow: Token![->] = input.parse()?;
            
            let content;
            let _ = syn::braced!(content in input);
            let action: proc_macro2::TokenStream = content.parse()?;

            variants.push(RuleVariant { pattern, action });

            if input.peek(Token![|]) {
                let _pipe: Token![|] = input.parse()?;
            } else {
                break;
            }
        }
        Ok(variants)
    }
}

// 3. Patterns (Manuell, da komplexe Precedence für Suffixes)
impl Parse for Pattern {
    fn parse(input: ParseStream) -> Result<Self> {
        // A. Base Pattern
        let mut base = if input.peek(LitStr) {
            Pattern::Lit(input.parse()?)
        } else if input.peek(token::Bracket) {
            let content;
            let _ = syn::bracketed!(content in input);
            Pattern::Bracketed(ParsePatternList::parse(&content)?)
        } else if input.peek(token::Brace) {
            let content;
            let _ = syn::braced!(content in input);
            Pattern::Braced(ParsePatternList::parse(&content)?)
        } else if input.peek(token::Paren) {
            let content;
            let _ = syn::parenthesized!(content in input);
            // In ( ... ) können Alternativen | stehen -> Group
            Pattern::Group(ParseGroupContent::parse(&content)?)
        } else if input.peek(kw::paren) {
            let _ = input.parse::<kw::paren>()?;
            let content;
            let _ = syn::parenthesized!(content in input);
            Pattern::Parenthesized(ParsePatternList::parse(&content)?)
        } else {
            // Rule Call: name:rule(arg)
            let binding = if input.peek2(Token![:]) {
                let b: Ident = input.parse()?;
                let _ = input.parse::<Token![:]>()?;
                Some(b)
            } else {
                None
            };

            let rule_name: Ident = input.parse()?;
            
            let mut args = Vec::new();
            if input.peek(token::Paren) {
                let content;
                let _ = syn::parenthesized!(content in input);
                while !content.is_empty() {
                    args.push(content.parse()?);
                    if content.peek(Token![,]) {
                        let _ = content.parse::<Token![,]>()?;
                    }
                }
            } else {
                // Erlaube rule calls ohne Klammern () falls keine Args da sind? 
                // Deine Grammatik nutzt "ident()", also Klammern sind Pflicht laut bisherigem Parser.
                // Wir erzwingen hier Klammern für Konsistenz mit alter Logik.
                let content;
                let _ = syn::parenthesized!(content in input); // Erwarte ()
            }

            Pattern::RuleCall { binding, rule_name, args }
        };

        // B. Suffixes (*, +, ?)
        loop {
            if input.peek(Token![?]) {
                let _ = input.parse::<Token![?]>()?;
                base = Pattern::Optional(Box::new(base));
            } else if input.peek(Token![*]) {
                let _ = input.parse::<Token![*]>()?;
                base = Pattern::Repeat(Box::new(base));
            } else if input.peek(Token![+]) {
                let _ = input.parse::<Token![+]>()?;
                base = Pattern::Plus(Box::new(base));
            } else {
                break;
            }
        }

        Ok(base)
    }
}

// Helpers
struct ParsePatternList;
impl ParsePatternList {
    fn parse(input: ParseStream) -> Result<Vec<Pattern>> {
        let mut seq = Vec::new();
        while !input.is_empty() {
            seq.push(input.parse()?);
        }
        Ok(seq)
    }
}

struct ParseGroupContent;
impl ParseGroupContent {
    fn parse(input: ParseStream) -> Result<Vec<Vec<Pattern>>> {
        let mut alts = Vec::new();
        loop {
            let mut seq = Vec::new();
            while !input.is_empty() && !input.peek(Token![|]) {
                seq.push(input.parse()?);
            }
            alts.push(seq);
            if input.peek(Token![|]) {
                let _ = input.parse::<Token![|]>()?;
            } else {
                break;
            }
        }
        Ok(alts)
    }
}
