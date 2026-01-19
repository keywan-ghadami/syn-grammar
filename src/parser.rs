use syn::parse::{Parse, ParseStream};
use syn::{Result, Token, token, LitStr, Ident, Type, parenthesized, bracketed, braced, Lit};
use derive_syn_parse::Parse;
use proc_macro2::TokenStream;

// Lokale Keywords für den Parser
mod kw {
    syn::custom_keyword!(grammar);
    syn::custom_keyword!(rule);
    syn::custom_keyword!(paren);
}

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

#[derive(Parse)]
pub struct InheritanceSpec {
    pub _colon: Token![:],
    pub name: Ident,
}

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
    pub fn parse_all(input: ParseStream) -> Result<Vec<Self>> {
        let mut rules = Vec::new();
        while !input.is_empty() {
            rules.push(input.parse()?);
        }
        Ok(rules)
    }
}

pub struct RuleVariant {
    pub pattern: Vec<Pattern>,
    pub action: TokenStream,
}

impl RuleVariant {
    pub fn parse_list(input: ParseStream) -> Result<Vec<Self>> {
        let mut variants = Vec::new();
        loop {
            let mut pattern = Vec::new();
            // Parsen bis zum Action-Pfeil '->' oder der nächsten Variante '|'
            while !input.peek(Token![->]) && !input.peek(Token![|]) {
                pattern.push(input.parse()?);
            }

            input.parse::<Token![->]>()?;
            let content;
            braced!(content in input);
            let action = content.parse()?;

            variants.push(RuleVariant { pattern, action });

            if input.peek(Token![|]) {
                input.parse::<Token![|]>()?;
            } else {
                break;
            }
        }
        Ok(variants)
    }
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

// Implementierung der Parse-Logik für Pattern (Atom + Suffixe)
impl Parse for Pattern {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut pat = parse_atom(input)?;

        // Suffix-Handling für Wiederholungen und Optionalität
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

/// Hilfsfunktion zum Parsen eines "Atoms" (kleinste Einheit ohne Suffix)
fn parse_atom(input: ParseStream) -> Result<Pattern> {
    if input.peek(LitStr) {
        Ok(Pattern::Lit(input.parse()?))
    } else if input.peek(token::Bracket) {
        let content;
        bracketed!(content in input);
        Ok(Pattern::Bracketed(parse_pattern_list(&content)?))
    } else if input.peek(token::Brace) {
        let content;
        braced!(content in input);
        Ok(Pattern::Braced(parse_pattern_list(&content)?))
    } else if input.peek(kw::paren) {
        input.parse::<kw::paren>()?;
        let content;
        parenthesized!(content in input);
        Ok(Pattern::Parenthesized(parse_pattern_list(&content)?))
    } else if input.peek(token::Paren) {
        let content;
        parenthesized!(content in input);
        Ok(Pattern::Group(parse_group_content(&content)?))
    } else {
        // RuleCall: Ident oder binding:Ident
        let binding = if input.peek2(Token![:]) {
            let id: Ident = input.parse()?;
            input.parse::<Token![:]>()?;
            Some(id)
        } else {
            None
        };

        let rule_name: Ident = input.parse()?;
        
        // Optionale Argumente in runden Klammern
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

fn parse_pattern_list(input: ParseStream) -> Result<Vec<Pattern>> {
    let mut list = Vec::new();
    while !input.is_empty() {
        list.push(input.parse()?);
    }
    Ok(list)
}

fn parse_group_content(input: ParseStream) -> Result<Vec<Vec<Pattern>>> {
    let mut alts = Vec::new();
    loop {
        let mut seq = Vec::new();
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
