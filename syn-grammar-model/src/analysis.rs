use crate::model::*;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use std::collections::HashSet;
use syn::{parse_quote, Result};

/// Collects all custom keywords from the grammar
pub fn collect_custom_keywords(grammar: &GrammarDefinition) -> HashSet<String> {
    let mut kws = HashSet::new();
    grammar
        .rules
        .iter()
        .flat_map(|r| &r.variants)
        .for_each(|v| collect_from_patterns(&v.pattern, &mut kws));
    kws
}

/// Result of analyzing a pattern sequence for a Cut operator (`=>`)
pub struct CutAnalysis<'a> {
    pub pre_cut: &'a [ModelPattern],
    pub post_cut: &'a [ModelPattern],
}

/// Checks if a sequence contains a Cut operator and splits it.
pub fn find_cut<'a>(patterns: &'a [ModelPattern]) -> Option<CutAnalysis<'a>> {
    let idx = patterns
        .iter()
        .position(|p| matches!(p, ModelPattern::Cut))?;
    Some(CutAnalysis {
        pre_cut: &patterns[0..idx],
        post_cut: &patterns[idx + 1..],
    })
}

/// Splits variants into recursive (starts with the rule name) and base cases.
pub fn split_left_recursive<'a>(
    rule_name: &Ident,
    variants: &'a [RuleVariant],
) -> (Vec<&'a RuleVariant>, Vec<&'a RuleVariant>) {
    let mut recursive = Vec::new();
    let mut base = Vec::new();

    for v in variants {
        if let Some(ModelPattern::RuleCall { rule_name: r, .. }) = v.pattern.first() {
            if r == rule_name {
                recursive.push(v);
                continue;
            }
        }
        base.push(v);
    }
    (recursive, base)
}

fn collect_from_patterns(patterns: &[ModelPattern], kws: &mut HashSet<String>) {
    for p in patterns {
        match p {
            ModelPattern::Lit(lit) => {
                let s = lit.value();
                if is_identifier(&s) && !is_rust_keyword(&s) {
                    kws.insert(s);
                }
            }
            ModelPattern::Group(alts) => {
                alts.iter().for_each(|alt| collect_from_patterns(alt, kws))
            }
            ModelPattern::Bracketed(s)
            | ModelPattern::Braced(s)
            | ModelPattern::Parenthesized(s) => collect_from_patterns(s, kws),
            ModelPattern::Optional(i) | ModelPattern::Repeat(i) | ModelPattern::Plus(i) => {
                collect_from_patterns(std::slice::from_ref(i), kws)
            }
            ModelPattern::Recover { body, sync, .. } => {
                collect_from_patterns(std::slice::from_ref(body), kws);
                collect_from_patterns(std::slice::from_ref(sync), kws);
            }
            _ => {}
        }
    }
}

pub fn collect_bindings(patterns: &[ModelPattern]) -> Vec<Ident> {
    let mut bindings = Vec::new();
    for p in patterns {
        match p {
            ModelPattern::RuleCall {
                binding: Some(b), ..
            } => bindings.push(b.clone()),
            ModelPattern::Repeat(inner) | ModelPattern::Plus(inner) => {
                if let ModelPattern::RuleCall {
                    binding: Some(b), ..
                } = &**inner
                {
                    bindings.push(b.clone());
                }
            }
            ModelPattern::Parenthesized(s)
            | ModelPattern::Bracketed(s)
            | ModelPattern::Braced(s) => {
                bindings.extend(collect_bindings(s));
            }
            ModelPattern::Recover { binding, body, .. } => {
                if let Some(b) = binding {
                    bindings.push(b.clone());
                } else {
                    bindings.extend(collect_bindings(std::slice::from_ref(body)));
                }
            }
            _ => {}
        }
    }
    bindings
}

/// Returns the token for syn::parse::<Token>() or peeking
pub fn resolve_token_type(
    lit: &syn::LitStr,
    custom_keywords: &HashSet<String>,
) -> Result<syn::Type> {
    let s = lit.value();

    if custom_keywords.contains(&s) {
        let ident = format_ident!("{}", s);
        return Ok(parse_quote!(kw::#ident));
    }

    if matches!(s.as_str(), "(" | ")" | "[" | "]" | "{" | "}") {
        return Err(syn::Error::new(lit.span(),
            format!("Invalid direct token literal: '{}'. Use paren(...), bracketed[...] or braced{{...}} instead.", s)));
    }

    // Check for numeric literals which are not supported as tokens
    if s.chars().next().is_some_and(|c| c.is_numeric()) {
        return Err(syn::Error::new(lit.span(),
            format!("Numeric literal '{}' cannot be used as a token. Use `int_lit` or similar parsers instead.", s)));
    }

    syn::parse_str::<syn::Type>(&format!("Token![{}]", s))
        .map_err(|_| syn::Error::new(lit.span(), format!("Invalid token literal: '{}'", s)))
}

/// Helper for UPO: Returns a TokenStream for input.peek(...)
pub fn get_simple_peek(
    pattern: &ModelPattern,
    kws: &HashSet<String>,
) -> Result<Option<TokenStream>> {
    match pattern {
        ModelPattern::Lit(lit) => {
            let token_type = resolve_token_type(lit, kws)?;
            Ok(Some(quote!(#token_type)))
        }
        ModelPattern::Bracketed(_) => Ok(Some(quote!(syn::token::Bracket))),
        ModelPattern::Braced(_) => Ok(Some(quote!(syn::token::Brace))),
        ModelPattern::Parenthesized(_) => Ok(Some(quote!(syn::token::Paren))),
        ModelPattern::Optional(inner) | ModelPattern::Repeat(inner) | ModelPattern::Plus(inner) => {
            get_simple_peek(inner, kws)
        }
        ModelPattern::Recover { body, .. } => get_simple_peek(body, kws),
        ModelPattern::Group(alts) => {
            if alts.len() == 1 {
                if let Some(first) = alts[0].first() {
                    get_simple_peek(first, kws)
                } else {
                    Ok(None)
                }
            } else {
                Ok(None)
            }
        }
        _ => Ok(None),
    }
}

/// Helper for UPO: Returns a unique string key for the start token
pub fn get_peek_token_string(patterns: &[ModelPattern]) -> Option<String> {
    match patterns.first() {
        Some(ModelPattern::Lit(l)) => Some(l.value()),
        Some(ModelPattern::Bracketed(_)) => Some("Bracket".to_string()),
        Some(ModelPattern::Braced(_)) => Some("Brace".to_string()),
        Some(ModelPattern::Parenthesized(_)) => Some("Paren".to_string()),
        Some(ModelPattern::Optional(inner))
        | Some(ModelPattern::Repeat(inner))
        | Some(ModelPattern::Plus(inner)) => get_peek_token_string(std::slice::from_ref(&**inner)),
        Some(ModelPattern::Recover { body, .. }) => {
            get_peek_token_string(std::slice::from_ref(&**body))
        }
        Some(ModelPattern::Group(alts)) => {
            if alts.len() == 1 {
                get_peek_token_string(&alts[0])
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Checks if a pattern can match the empty string (epsilon).
/// Used to determine if it is safe to skip a pattern based on a failed peek.
pub fn is_nullable(pattern: &ModelPattern) -> bool {
    match pattern {
        ModelPattern::Cut => true,
        ModelPattern::Lit(_) => false,
        // Conservative assumption: Rule calls might be nullable.
        // To be safe, we assume they are, preventing unsafe peek optimizations.
        ModelPattern::RuleCall { .. } => true,
        ModelPattern::Group(alts) => alts.iter().any(|seq| seq.iter().all(is_nullable)),
        ModelPattern::Bracketed(_) | ModelPattern::Braced(_) | ModelPattern::Parenthesized(_) => {
            false
        }
        ModelPattern::Optional(_) => true,
        ModelPattern::Repeat(_) => true,
        ModelPattern::Plus(inner) => is_nullable(inner),
        ModelPattern::Recover { .. } => true,
    }
}

fn is_identifier(s: &str) -> bool {
    s.chars()
        .next()
        .is_some_and(|c| c.is_alphabetic() || c == '_')
        && s.chars().all(|c| c.is_alphanumeric() || c == '_')
}

fn is_rust_keyword(s: &str) -> bool {
    matches!(
        s,
        "fn" | "let"
            | "struct"
            | "enum"
            | "if"
            | "else"
            | "while"
            | "loop"
            | "for"
            | "match"
            | "return"
            | "pub"
            | "mod"
            | "use"
            | "type"
            | "trait"
            | "impl"
            | "const"
            | "static"
            | "mut"
            | "unsafe"
            | "extern"
            | "ref"
            | "self"
            | "Self"
            | "super"
            | "crate"
            | "async"
            | "await"
            | "where"
            | "move"
            | "true"
            | "false"
            | "in"
            | "as"
            | "dyn"
    )
}
