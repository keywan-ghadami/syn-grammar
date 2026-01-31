// Moved from macros/src/parser.rs
use syn::parse::{Parse, ParseStream};
use syn::{Result, Token, token, LitStr, Ident, Type, Lit};
use proc_macro2::TokenStream;

mod rt {
    use syn::parse::ParseStream;
    use syn::Result;
    use syn::ext::IdentExt; 
    use syn::parse::discouraged::Speculative;

    pub fn attempt<T>(input: ParseStream, parser: impl FnOnce(ParseStream) -> Result<T>) -> Result<Option<T>> {
        let fork = input.fork();
        match parser(&fork) {
            Ok(res) => {
                input.advance_to(&fork);
                Ok(Some(res))
            }
            Err(_) => Ok(None),
        }
    }

    pub fn parse_ident(input: ParseStream) -> Result<syn::Ident> {
        input.call(syn::Ident::parse_any)
    }
}

mod kw {
    syn::custom_keyword!(grammar);
    syn::custom_keyword!(rule);
    syn::custom_keyword!(paren);
}

pub struct GrammarDefinition {
    pub name: Ident,
    pub inherits: Option<InheritanceSpec>,
    pub rules: Vec<Rule>,
}

impl Parse for GrammarDefinition {
    fn parse(input: ParseStream) -> Result<Self> {
        let _ = input.parse::<kw::grammar>()?;
        let name = rt::parse_ident(input)?;
        
        let inherits = if input.peek(Token![:]) {
            Some(input.parse::<InheritanceSpec>()?)
        } else {
            None
        };

        let content;
        let _ = syn::braced!(content in input);
        let rules = Rule::parse_all(&content)?;

        Ok(GrammarDefinition { name, inherits, rules })
    }
}

pub struct InheritanceSpec {
    pub name: Ident,
}

impl Parse for InheritanceSpec {
    fn parse(input: ParseStream) -> Result<Self> {
        let _ = input.parse::<Token![:]>()?;
        let name = rt::parse_ident(input)?;
        Ok(InheritanceSpec { name })
    }
}

pub struct RuleParameter {
    pub name: Ident,
    pub _colon: Token![:],
    pub ty: Type,
}

impl Parse for RuleParameter {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(RuleParameter {
            name: input.parse()?,
            _colon: input.parse()?,
            ty: input.parse()?,
        })
    }
}

pub struct Rule {
    pub is_pub: Option<Token![pub]>,
    pub name: Ident,
    pub params: Vec<RuleParameter>,
    pub return_type: Type,
    pub variants: Vec<RuleVariant>,
}

impl Parse for Rule {
    fn parse(input: ParseStream) -> Result<Self> {
        let is_pub = if input.peek(Token![pub]) {
            Some(input.parse()?)
        } else {
            None
        };

        let _ = input.parse::<kw::rule>()?;
        let name = rt::parse_ident(input)?;

        let params = if input.peek(token::Paren) {
            let content;
            syn::parenthesized!(content in input);
            let mut params = Vec::new();
            while !content.is_empty() {
                params.push(content.parse()?);
                if content.peek(Token![,]) {
                    let _ = content.parse::<Token![,]>()?;
                }
            }
            params
        } else {
            Vec::new()
        };

        let _ = input.parse::<Token![->]>()?;
        let return_type = input.parse::<Type>()?;
        let _ = input.parse::<Token![=]>()?;
        
        let variants = RuleVariant::parse_list(input)?;

        Ok(Rule { is_pub, name, params, return_type, variants })
    }
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
            while !input.peek(Token![->]) && !input.peek(Token![|]) {
                pattern.push(input.parse()?);
            }

            let _ = input.parse::<Token![->]>()?;
            
            let content;
            syn::braced!(content in input);
            let action = content.parse()?;

            variants.push(RuleVariant { pattern, action });

            if input.peek(Token![|]) {
                let _ = input.parse::<Token![|]>()?;
            } else {
                break;
            }
        }
        Ok(variants)
    }
}

#[derive(Debug, Clone)]
pub enum Pattern {
    Cut,
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

impl Parse for Pattern {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut pat = parse_atom(input)?;

        loop {
            if input.peek(Token![*]) {
                let _ = input.parse::<Token![*]>()?;
                pat = Pattern::Repeat(Box::new(pat));
            } else if input.peek(Token![+]) {
                let _ = input.parse::<Token![+]>()?;
                pat = Pattern::Plus(Box::new(pat));
            } else if input.peek(Token![?]) {
                let _ = input.parse::<Token![?]>()?;
                pat = Pattern::Optional(Box::new(pat));
            } else {
                break;
            }
        }
        Ok(pat)
    }
}

fn parse_atom(input: ParseStream) -> Result<Pattern> {
    if input.peek(Token![=>]) {
        let _ = input.parse::<Token![=>]>()?;
        Ok(Pattern::Cut)
    } else if input.peek(LitStr) {
        Ok(Pattern::Lit(input.parse()?))
    } else if input.peek(token::Bracket) {
        let content;
        syn::bracketed!(content in input);
        Ok(Pattern::Bracketed(parse_pattern_list(&content)?))
    } else if input.peek(token::Brace) {
        let content;
        syn::braced!(content in input);
        Ok(Pattern::Braced(parse_pattern_list(&content)?))
    } else if input.peek(kw::paren) {
        let _ = input.parse::<kw::paren>()?;
        let content;
        syn::parenthesized!(content in input);
        Ok(Pattern::Parenthesized(parse_pattern_list(&content)?))
    } else if input.peek(token::Paren) {
        let content;
        syn::parenthesized!(content in input);
        Ok(Pattern::Group(parse_group_content(&content)?))
    } else {
        if let Some(binding) = rt::attempt(input, |input| {
            let id: Ident = input.parse()?;
            let _ = input.parse::<Token![:]>()?;
            Ok(id)
        })? {
            let rule_name: Ident = rt::parse_ident(input)?;
            let args = parse_args(input)?;
            Ok(Pattern::RuleCall { binding: Some(binding), rule_name, args })
        } else {
            let rule_name: Ident = rt::parse_ident(input)?;
            let args = parse_args(input)?;
            Ok(Pattern::RuleCall { binding: None, rule_name, args })
        }
    }
}

fn parse_args(input: ParseStream) -> Result<Vec<Lit>> {
    let mut args = Vec::new();
    if input.peek(token::Paren) {
        let content;
        syn::parenthesized!(content in input);
        while !content.is_empty() {
            args.push(content.parse()?);
            if content.peek(Token![,]) {
                let _ = content.parse::<Token![,]>()?;
            }
        }
    }
    Ok(args)
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
            let _ = input.parse::<Token![|]>()?;
        } else {
            break;
        }
    }
    Ok(alts)
}
