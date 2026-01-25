use syn::parse::{Parse, ParseStream};
use syn::{Result, Token, token, LitStr, Ident, Type, parenthesized, bracketed, braced, Lit};
use derive_syn_parse::Parse;
use proc_macro2::TokenStream;

// Definition der Custom-Keywords für die DSL
mod kw {
    syn::custom_keyword!(grammar);
    syn::custom_keyword!(rule);
    syn::custom_keyword!(paren);
}

// --- Top-Level Grammatik Definition ---
// Syntax: grammar Name [: Parent] { ... }
#[derive(Parse)]
pub struct GrammarDefinition {
    pub _kw: kw::grammar,
    pub name: Ident,
    #[peek(Token![:])]
    pub inherits: Option<InheritanceSpec>,
    #[brace]
    pub _brace: token::Brace,
    #[inside(_brace)]
    #[call(Rule::parse_all)]
    pub rules: Vec<Rule>,
}

// Hilfsstruktur für Vererbung ": ParentName"
#[derive(Parse)]
pub struct InheritanceSpec {
    pub _colon: Token![:],
    pub name: Ident,
}

// --- Regel Definition ---
// Syntax: [pub] rule Name -> Type = ...
#[derive(Parse)]
pub struct Rule {
    #[peek(Token![pub])]
    pub is_pub: Option<Token![pub]>,
    pub _kw: kw::rule,
    pub name: Ident,
    pub _arrow: Token![->],
    pub return_type: Type,
    pub _eq: Token![=],
    #[call(RuleVariant::parse_list)]
    pub variants: Vec<RuleVariant>,
}

impl Rule {
    /// Parsed alle Regeln innerhalb des grammar { ... } Blocks
    pub fn parse_all(input: ParseStream) -> Result<Vec<Self>> {
        let mut rules = Vec::new();
        while !input.is_empty() {
            rules.push(input.parse()?);
        }
        Ok(rules)
    }
}

// --- Varianten und Action ---
// Syntax: Pattern ... -> { ActionCode }
pub struct RuleVariant {
    pub pattern: Vec<Pattern>,
    pub action: TokenStream,
}

impl RuleVariant {
    /// Parsed eine Liste von Varianten, getrennt durch '|'
    pub fn parse_list(input: ParseStream) -> Result<Vec<Self>> {
        let mut variants = Vec::new();
        loop {
            let mut pattern = Vec::new();
            // Parsen bis zum Action-Pfeil '->' oder der nächsten Variante '|'
            // (Dies erlaubt leere Patterns, falls nötig, aber meistens steht da was)
            while !input.peek(Token![->]) && !input.peek(Token![|]) {
                pattern.push(input.parse()?);
            }

            // Der Pfeil ist Pflicht vor dem Code-Block
            input.parse::<Token![->]>()?;
            
            let content;
            braced!(content in input); // Der Action-Code ist in geschweiften Klammern
            let action = content.parse()?;

            variants.push(RuleVariant { pattern, action });

            // Gibt es weitere Varianten?
            if input.peek(Token![|]) {
                input.parse::<Token![|]>()?;
            } else {
                break;
            }
        }
        Ok(variants)
    }
}

// --- Pattern (Der komplexe Teil) ---
#[derive(Debug, Clone)]
pub enum Pattern {
    Cut,
    Lit(LitStr),
    RuleCall {
        binding: Option<Ident>,
        rule_name: Ident,
        args: Vec<Lit>, 
    },
    Group(Vec<Vec<Pattern>>), // Für ( A | B )
    Bracketed(Vec<Pattern>),  // Für [ ... ] -> syn::bracketed!
    Braced(Vec<Pattern>),     // Für { ... } -> syn::braced!
    Parenthesized(Vec<Pattern>), // Für paren(...) -> syn::parenthesized!
    Optional(Box<Pattern>),   // Suffix ?
    Repeat(Box<Pattern>),     // Suffix *
    Plus(Box<Pattern>),       // Suffix +
}

// Implementierung der Parse-Logik für Pattern (Atom + Suffixe)
impl Parse for Pattern {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut pat = parse_atom(input)?;

        // Suffix-Handling für Wiederholungen und Optionalität (?, *, +)
        loop {
            if input.peek(Token![*]) {
                input.parse::<Token![*]>()?;
                pat = Pattern::Repeat(Box::new(pat));
            } else if input.peek(Token![+]) {
                input.parse::<Token![+]>()?;
                pat = Pattern::Plus(Box::new(pat));
            } else if input.peek(Token![?]) {
                input.parse::<Token![?]>()?;
                pat = Pattern::Optional(Box::new(pat));
            } else {
                break;
            }
        }
        Ok(pat)
    }
}

/// Parsed den "Kern" eines Patterns ohne Suffixe
fn parse_atom(input: ParseStream) -> Result<Pattern> {
    if input.peek(Token![=>]) {
        input.parse::<Token![=>]>()?;
        Ok(Pattern::Cut)
    } else if input.peek(LitStr) {
        // String Literal: "foo"
        Ok(Pattern::Lit(input.parse()?))
    } else if input.peek(token::Bracket) {
        // [ ... ] -> Bracketed
        let content;
        bracketed!(content in input);
        Ok(Pattern::Bracketed(parse_pattern_list(&content)?))
    } else if input.peek(token::Brace) {
        // { ... } -> Braced
        let content;
        braced!(content in input);
        Ok(Pattern::Braced(parse_pattern_list(&content)?))
    } else if input.peek(kw::paren) {
        // paren( ... ) -> Parenthesized (Expliziter Scope-Export)
        input.parse::<kw::paren>()?;
        let content;
        parenthesized!(content in input);
        Ok(Pattern::Parenthesized(parse_pattern_list(&content)?))
    } else if input.peek(token::Paren) {
        // ( ... ) -> Grouping (Logische Gruppierung ohne Token-Konsum im Output)
        let content;
        parenthesized!(content in input);
        Ok(Pattern::Group(parse_group_content(&content)?))
    } else {
        // RuleCall oder Binding: name oder binding:name
        let binding = if input.peek2(Token![:]) {
            let id: Ident = input.parse()?;
            input.parse::<Token![:]>()?;
            Some(id)
        } else {
            None
        };

        let rule_name: Ident = input.parse()?;
        
        // Optionale Argumente in runden Klammern: rule(1, "foo")
        let mut args = Vec::new();
        if input.peek(token::Paren) {
            let content;
            parenthesized!(content in input);
            while !content.is_empty() {
                args.push(content.parse()?);
                if content.peek(Token![,]) {
                    content.parse::<Token![,]>()?;
                }
            }
        }

        Ok(Pattern::RuleCall { binding, rule_name, args })
    }
}

/// Helper: Parsed eine Sequenz von Patterns (für Inhalte von Klammern)
fn parse_pattern_list(input: ParseStream) -> Result<Vec<Pattern>> {
    let mut list = Vec::new();
    while !input.is_empty() {
        list.push(input.parse()?);
    }
    Ok(list)
}

/// Helper: Parsed Gruppen-Inhalte mit Alternativen (für (A | B))
fn parse_group_content(input: ParseStream) -> Result<Vec<Vec<Pattern>>> {
    let mut alts = Vec::new();
    loop {
        let mut seq = Vec::new();
        // Parsen bis zum Ende oder bis zum '|'
        while !input.is_empty() && !input.peek(Token![|]) {
            seq.push(input.parse()?);
        }
        alts.push(seq);
        if input.peek(Token![|]) {
            input.parse::<Token![|]>()?;
        } else {
            break;
        }
    }
    Ok(alts)
}
