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
                // Try to tokenize the string literal to find identifiers
                if let Ok(ts) = syn::parse_str::<proc_macro2::TokenStream>(&s) {
                    for token in ts {
                        if let proc_macro2::TokenTree::Ident(ident) = token {
                            let s = ident.to_string();
                            // If syn accepts it as an Ident, it's a candidate.
                            // We rely on syn::parse_str::<syn::Ident> to filter out reserved keywords.
                            // We also exclude "_" because it cannot be a struct name for custom_keyword!.
                            if s != "_" && syn::parse_str::<syn::Ident>(&s).is_ok() {
                                kws.insert(s);
                            }
                        }
                    }
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

/// Returns the sequence of tokens for syn::parse::<Token>()
///
/// This handles:
/// 1. Custom keywords (e.g. "my_kw")
/// 2. Single tokens (e.g. "->", "==")
/// 3. Multi-token sequences (e.g. "?.", "@detached")
pub fn resolve_token_types(
    lit: &syn::LitStr,
    custom_keywords: &HashSet<String>,
) -> Result<Vec<syn::Type>> {
    let s = lit.value();

    // 1. Check for exact custom keyword match
    if custom_keywords.contains(&s) {
        let ident = format_ident!("{}", s);
        return Ok(vec![parse_quote!(kw::#ident)]);
    }

    // 2. Check for forbidden direct tokens
    if matches!(s.as_str(), "(" | ")" | "[" | "]" | "{" | "}") {
        return Err(syn::Error::new(
            lit.span(),
            format!(
                "Invalid direct token literal: '{}'. Use paren(...), [...] or {{...}} instead.",
                s
            ),
        ));
    }

    // 3. Check for boolean/numeric literals
    if s == "true" || s == "false" {
        return Err(syn::Error::new(
            lit.span(),
            format!(
                "Boolean literal '{}' cannot be used as a token. Use `lit_bool` parser instead.",
                s
            ),
        ));
    }
    if s.chars().next().is_some_and(|c| c.is_numeric()) {
        return Err(syn::Error::new(lit.span(),
            format!("Numeric literal '{}' cannot be used as a token. Use `integer` or `lit_int` parsers instead.", s)));
    }

    // 4. Try parsing as a single Token![...] type
    // This handles standard operators like "->", "==", etc.
    if let Ok(ty) = syn::parse_str::<syn::Type>(&format!("Token![{}]", s)) {
        return Ok(vec![ty]);
    }

    // 5. If single token failed, try splitting into multiple tokens
    // e.g. "?." -> Token![?] + Token![.]
    // e.g. "@detached" -> Token![@] + kw::detached
    let ts: proc_macro2::TokenStream = syn::parse_str(&s)
        .map_err(|_| syn::Error::new(lit.span(), format!("Invalid token literal: '{}'", s)))?;

    let mut types = Vec::new();
    for token in ts {
        match token {
            proc_macro2::TokenTree::Punct(p) => {
                let c = p.as_char();
                let ty: syn::Type = syn::parse_str(&format!("Token![{}]", c)).map_err(|_| {
                    syn::Error::new(
                        lit.span(),
                        format!("Cannot map punctuation '{}' to Token!", c),
                    )
                })?;
                types.push(ty);
            }
            proc_macro2::TokenTree::Ident(i) => {
                let s = i.to_string();
                if custom_keywords.contains(&s) {
                    let ident = format_ident!("{}", s);
                    types.push(parse_quote!(kw::#ident));
                } else {
                    // Try as standard token (e.g. keyword)
                    let ty: syn::Type = syn::parse_str(&format!("Token![{}]", s)).map_err(|_| {
                        syn::Error::new(
                            lit.span(),
                            format!(
                                "Identifier '{}' is not a custom keyword and not a valid Token!",
                                s
                            ),
                        )
                    })?;
                    types.push(ty);
                }
            }
            _ => {
                return Err(syn::Error::new(
                    lit.span(),
                    "Literal contains unsupported token tree (Group or Literal)",
                ))
            }
        }
    }

    if types.is_empty() {
        return Err(syn::Error::new(
            lit.span(),
            "Empty string literal is not supported.",
        ));
    }

    Ok(types)
}

/// Helper for UPO: Returns a TokenStream for input.peek(...)
pub fn get_simple_peek(
    pattern: &ModelPattern,
    kws: &HashSet<String>,
) -> Result<Option<TokenStream>> {
    match pattern {
        ModelPattern::Lit(lit) => {
            let token_types = resolve_token_types(lit, kws)?;
            // Peek the first token
            if let Some(first_type) = token_types.first() {
                Ok(Some(quote!(#first_type)))
            } else {
                Ok(None)
            }
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
