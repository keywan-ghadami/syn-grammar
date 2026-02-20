// Moved from macros/src/parser.rs
use proc_macro2::TokenStream;
use syn::parse::{Parse, ParseStream};
use syn::{token, Attribute, Generics, Ident, ItemUse, Lit, Result, Token, Type};

mod rt {
    use syn::ext::IdentExt;
    use syn::parse::discouraged::Speculative;
    use syn::parse::ParseStream;
    use syn::Result;

    pub fn attempt<T>(
        input: ParseStream,
        parser: impl FnOnce(ParseStream) -> Result<T>,
    ) -> Result<Option<T>> {
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

pub mod kw {
    syn::custom_keyword!(grammar);
    syn::custom_keyword!(rule);
    syn::custom_keyword!(paren);
    syn::custom_keyword!(recover);
    syn::custom_keyword!(peek);
    syn::custom_keyword!(not);
    syn::custom_keyword!(until);
}

pub struct GrammarDefinition {
    pub name: Ident,
    pub inherits: Option<InheritanceSpec>,
    pub uses: Vec<ItemUse>,
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

        let mut uses = Vec::new();
        while content.peek(Token![use]) {
            uses.push(content.parse()?);
        }

        let rules = Rule::parse_all(&content)?;

        Ok(GrammarDefinition {
            name,
            inherits,
            uses,
            rules,
        })
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
    pub ty: Option<Type>,
}

impl Parse for RuleParameter {
    fn parse(input: ParseStream) -> Result<Self> {
        let name: Ident = input.parse()?;
        let ty = if input.peek(Token![:]) {
            let _ = input.parse::<Token![:]>()?;
            Some(input.parse()?)
        } else {
            None
        };
        Ok(RuleParameter { name, ty })
    }
}

pub struct Rule {
    pub attrs: Vec<Attribute>,
    pub is_pub: Option<Token![pub]>,
    pub name: Ident,
    pub generics: Generics,
    pub params: Vec<RuleParameter>,
    pub return_type: Type,
    pub variants: Vec<RuleVariant>,
}

impl Parse for Rule {
    fn parse(input: ParseStream) -> Result<Self> {
        let attrs = Attribute::parse_outer(input)?;

        let is_pub = if input.peek(Token![pub]) {
            Some(input.parse()?)
        } else {
            None
        };

        let _ = input.parse::<kw::rule>()?;
        let name = rt::parse_ident(input)?;

        // Parse generics if present (e.g., <T, U>)
        let generics: Generics = input.parse()?;

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

        Ok(Rule {
            attrs,
            is_pub,
            name,
            generics,
            params,
            return_type,
            variants,
        })
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
pub enum Argument {
    Positional(Pattern),
    Named(Ident, Pattern),
}

impl Parse for Argument {
    fn parse(input: ParseStream) -> Result<Self> {
        // Check for Named: Ident = ...
        // But Pattern can also start with Ident.
        // Ambiguity: `x` could be a rule call `x` or named arg `x = ...`.
        // We peek for `=` to distinguish.

        if input.peek(Ident) && input.peek2(Token![=]) {
            let name: Ident = input.parse()?;
            let _ = input.parse::<Token![=]>()?;
            let val: Pattern = input.parse()?;
            Ok(Argument::Named(name, val))
        } else {
            Ok(Argument::Positional(input.parse()?))
        }
    }
}

#[derive(Debug, Clone)]
pub enum Pattern {
    Cut(Token![=>]),
    Lit {
        binding: Option<Ident>,
        lit: Lit,
    },
    RuleCall {
        binding: Option<Ident>,
        rule_name: Ident,
        generics: Vec<Type>, // Added generics support
        args: Vec<Argument>, // Changed from Vec<Pattern>
    },
    Group(Vec<Vec<Pattern>>, token::Paren),
    Bracketed(Vec<Pattern>, token::Bracket),
    Braced(Vec<Pattern>, token::Brace),
    Parenthesized(Vec<Pattern>, kw::paren, token::Paren),
    Optional(Box<Pattern>, Token![?]),
    Repeat(Box<Pattern>, Token![*]),
    Plus(Box<Pattern>, Token![+]),
    SpanBinding(Box<Pattern>, Ident, Token![@]),
    Recover {
        binding: Option<Ident>,
        body: Box<Pattern>,
        sync: Box<Pattern>,
        kw_token: kw::recover,
    },
    Peek(Box<Pattern>, kw::peek),
    Not(Box<Pattern>, kw::not),
    Until {
        binding: Option<Ident>,
        pattern: Box<Pattern>,
        kw_token: kw::until,
    },
}

impl Parse for Pattern {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut pat = parse_atom(input)?;

        loop {
            if input.peek(Token![*]) {
                let token = input.parse::<Token![*]>()?;
                pat = Pattern::Repeat(Box::new(pat), token);
            } else if input.peek(Token![+]) {
                let token = input.parse::<Token![+]>()?;
                pat = Pattern::Plus(Box::new(pat), token);
            } else if input.peek(Token![?]) {
                let token = input.parse::<Token![?]>()?;
                pat = Pattern::Optional(Box::new(pat), token);
            } else if input.peek(Token![@]) {
                let token = input.parse::<Token![@]>()?;
                let ident = input.parse::<Ident>()?;
                pat = Pattern::SpanBinding(Box::new(pat), ident, token);
            } else {
                break;
            }
        }
        Ok(pat)
    }
}

fn parse_atom(input: ParseStream) -> Result<Pattern> {
    // 1. Check for binding
    let binding = rt::attempt(input, |input| {
        let id: Ident = input.parse()?;
        let _ = input.parse::<Token![:]>()?;
        Ok(id)
    })?;

    if input.peek(Token![=>]) {
        if binding.is_some() {
            return Err(input.error("Cut operator cannot be bound."));
        }
        let token = input.parse::<Token![=>]>()?;
        Ok(Pattern::Cut(token))
    } else if input.peek(Lit) {
        Ok(Pattern::Lit {
            binding,
            lit: input.parse()?,
        })
    } else if input.peek(token::Bracket) {
        if binding.is_some() {
            return Err(input.error("Bracketed groups cannot be bound directly."));
        }
        let content;
        let token = syn::bracketed!(content in input);
        Ok(Pattern::Bracketed(parse_pattern_list(&content)?, token))
    } else if input.peek(token::Brace) {
        if binding.is_some() {
            return Err(input.error("Braced groups cannot be bound directly."));
        }
        let content;
        let token = syn::braced!(content in input);
        Ok(Pattern::Braced(parse_pattern_list(&content)?, token))
    } else if input.peek(kw::paren) {
        if binding.is_some() {
            return Err(input.error("Parenthesized groups cannot be bound directly."));
        }
        let kw = input.parse::<kw::paren>()?;
        let content;
        let token = syn::parenthesized!(content in input);
        Ok(Pattern::Parenthesized(
            parse_pattern_list(&content)?,
            kw,
            token,
        ))
    } else if input.peek(token::Paren) {
        if binding.is_some() {
            return Err(input.error("Groups cannot be bound directly."));
        }
        let content;
        let token = syn::parenthesized!(content in input);
        Ok(Pattern::Group(parse_group_content(&content)?, token))
    } else if input.peek(kw::recover) {
        let kw_token = input.parse::<kw::recover>()?;
        let content;
        syn::parenthesized!(content in input);
        let body = content.parse()?;
        let _ = content.parse::<Token![,]>()?;
        let sync = content.parse()?;
        Ok(Pattern::Recover {
            binding,
            body: Box::new(body),
            sync: Box::new(sync),
            kw_token,
        })
    } else if input.peek(kw::peek) {
        if binding.is_some() {
            return Err(input.error("Peek cannot be bound."));
        }
        let kw_token = input.parse::<kw::peek>()?;
        let content;
        syn::parenthesized!(content in input);
        let inner = content.parse()?;
        Ok(Pattern::Peek(Box::new(inner), kw_token))
    } else if input.peek(kw::not) {
        if binding.is_some() {
            return Err(input.error("Not cannot be bound."));
        }
        let kw_token = input.parse::<kw::not>()?;
        let content;
        syn::parenthesized!(content in input);
        let inner = content.parse()?;
        Ok(Pattern::Not(Box::new(inner), kw_token))
    } else if input.peek(kw::until) {
        // until returns a TokenStream, so it can be bound.
        let kw_token = input.parse::<kw::until>()?;
        let content;
        syn::parenthesized!(content in input);
        let pattern = content.parse()?;
        Ok(Pattern::Until {
            binding,
            pattern: Box::new(pattern),
            kw_token,
        })
    } else {
        let rule_name: Ident = rt::parse_ident(input)?;
        let mut last_span = rule_name.span();

        // Parse generics: rule<T, U>
        let generics = if input.peek(Token![<]) {
            let _ = input.parse::<Token![<]>()?;
            let mut types = Vec::new();
            loop {
                types.push(input.parse::<Type>()?);
                if input.peek(Token![,]) {
                    let _ = input.parse::<Token![,]>()?;
                    if input.peek(Token![>]) {
                        break;
                    }
                } else {
                    break;
                }
            }
            let gt_token = input.parse::<Token![>]>()?;
            last_span = gt_token.span;
            types
        } else {
            Vec::new()
        };

        let args = if input.peek(token::Paren) {
            let paren_span = input.cursor().span();
            if spans_are_contiguous(last_span, paren_span) {
                parse_args(input)?
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        Ok(Pattern::RuleCall {
            binding,
            rule_name,
            generics,
            args,
        })
    }
}

fn parse_args(input: ParseStream) -> Result<Vec<Argument>> {
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

fn spans_are_contiguous(first: proc_macro2::Span, second: proc_macro2::Span) -> bool {
    let first_end = first.end();
    let second_start = second.start();

    if first_end.line != second_start.line {
        return false;
    }

    first_end.column == second_start.column
}
