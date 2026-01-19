use crate::model::*;
use quote::{quote, quote_spanned, format_ident};
use proc_macro2::TokenStream;
use std::collections::HashSet;
use syn::{Result, parse_quote};
use itertools::Itertools; 

/// Generiert den vollständigen Rust-Code für eine Grammatik-Definition.
pub fn generate_rust(grammar: GrammarDefinition) -> Result<TokenStream> {
    let grammar_name = &grammar.name;
    let custom_keywords = collect_custom_keywords(&grammar);

    // Custom Keywords Definition
    let kw_defs = (!custom_keywords.is_empty()).then(|| {
        let defs = custom_keywords.iter().map(|k| {
            let ident = format_ident!("{}", k);
            quote! { syn::custom_keyword!(#ident); }
        });
        quote! { pub mod kw { #(#defs)* } }
    });

    // Vererbung
    let inheritance = grammar.inherits.as_ref().map(|parent| {
        quote! { use super::#parent::*; }
    });

    // Regeln generieren
    let rules = grammar.rules.iter()
        .map(|r| generate_rule(r, &custom_keywords))
        .collect::<Result<Vec<_>>>()?;

    Ok(quote! {
        #![allow(unused_imports, unused_variables, dead_code, unused_braces)]
        pub const GRAMMAR_NAME: &str = stringify!(#grammar_name);

        use syn::parse::{Parse, ParseStream};
        use syn::Result;
        use syn::Token;
        use syn::ext::IdentExt; 
        use syn_grammar::rt; 

        #kw_defs
        #inheritance
        
        #(#rules)*
    })
}

fn generate_rule(rule: &Rule, custom_keywords: &HashSet<String>) -> Result<TokenStream> {
    let fn_name = format_ident!("parse_{}", rule.name);
    let ret_type = &rule.return_type;
    let vis = if rule.is_pub { quote!(pub) } else { quote!() };
    let body = generate_variants(&rule.variants, true, custom_keywords)?; 

    Ok(quote! {
        #vis fn #fn_name(input: ParseStream) -> Result<#ret_type> {
            #body
        }
    })
}

fn generate_variants(
    variants: &[RuleVariant], 
    is_top_level: bool,
    custom_keywords: &HashSet<String>
) -> Result<TokenStream> {
    if variants.is_empty() {
        return Ok(quote! { Err(input.error("No variants defined")) });
    }

    let arms = variants.iter().map(|variant| {
        let logic = generate_sequence(&variant.pattern, &variant.action, custom_keywords)?;
        
        // Nutzt ModelPattern für den Lookahead-Check
        let peek_token = variant.pattern.first()
            .and_then(|f| get_simple_peek(f, custom_keywords).ok().flatten());

        Ok(match peek_token {
            Some(token) => quote! { if input.peek(#token) { #logic } },
            None => quote! { if let Some(res) = rt::attempt(input, |input| { #logic })? { Ok(res) } }
        })
    }).collect::<Result<Vec<_>>>()?;

    let error_msg = if is_top_level { 
        "No matching rule variant found" 
    } else { 
        "No matching variant in group" 
    };

    // Baut die if-else if-else Kette
    Ok(quote! {
        #(#arms else)* {
            Err(input.error(#error_msg))
        }
    })
}

fn generate_sequence(patterns: &[ModelPattern], action: &TokenStream, kws: &HashSet<String>) -> Result<TokenStream> {
    let steps = generate_sequence_steps(patterns, kws)?;
    Ok(quote! { { #steps Ok(#action) } })
}

fn generate_pattern_step(pattern: &ModelPattern, kws: &HashSet<String>) -> Result<TokenStream> {
    let span = pattern.span();

    // Lokales Hilfsmakro für Suffixe (*, +)
    macro_rules! attempt_op {
        ($inner:expr, $wrapper:ident) => {{
            let inner_logic = generate_pattern_step($inner, kws)?;
            match get_simple_peek($inner, kws)? {
                Some(peek) => quote_spanned! {span=> $wrapper input.peek(#peek) { #inner_logic } },
                None => quote_spanned! {span=> $wrapper let Some(_) = rt::attempt(input, |input| { #inner_logic Ok(()) })? {} }
            }
        }};
    }

    match pattern {
        ModelPattern::Lit(lit) => {
            let token_type = resolve_token_type(lit, kws)?;
            Ok(quote_spanned! {span=> let _ = input.parse::<#token_type>()?; })
        },
        ModelPattern::RuleCall { binding, rule_name, args } => {
            let func_call = if is_builtin(rule_name) {
                map_builtin(rule_name)
            } else {
                let f = format_ident!("parse_{}", rule_name);
                if args.is_empty() { quote!(#f(input)?) } else { quote!(#f(input, #(#args),*)?) }
            };
            Ok(if let Some(bind) = binding {
                quote_spanned! {span=> let #bind = #func_call; }
            } else {
                quote_spanned! {span=> let _ = #func_call; }
            })
        },
        ModelPattern::Optional(inner) => {
            let inner_logic = generate_pattern_step(inner, kws)?;
            Ok(match get_simple_peek(inner, kws)? {
                Some(peek) => quote_spanned! {span=> if input.peek(#peek) { #inner_logic } },
                None => quote_spanned! {span=> let _ = rt::attempt(input, |input| { #inner_logic Ok(()) })?; }
            })
        },
        ModelPattern::Repeat(inner) => Ok(attempt_op!(inner, while)),
        ModelPattern::Plus(inner) => {
            let first = generate_pattern_step(inner, kws)?;
            let rest = attempt_op!(inner, while);
            Ok(quote_spanned! {span=> #first #rest })
        },
        ModelPattern::Group(alts) => {
            let temp_variants = alts.iter()
                .map(|pat_seq| RuleVariant { pattern: pat_seq.clone(), action: quote!({}) })
                .collect_vec();
            let variant_logic = generate_variants(&temp_variants, false, kws)?;
            Ok(quote_spanned! {span=> { #variant_logic }?; })
        },
        ModelPattern::Bracketed(s) | ModelPattern::Braced(s) | ModelPattern::Parenthesized(s) => {
            let macro_name = match pattern {
                ModelPattern::Bracketed(_) => quote!(bracketed),
                ModelPattern::Braced(_) => quote!(braced),
                _ => quote!(parenthesized),
            };
            let inner_logic = generate_sequence_steps(s, kws)?;
            Ok(quote_spanned! {span=> {
                let content;
                let _ = syn::#macro_name!(content in input);
                let input = &content;
                #inner_logic
            }})
        },
    }
}

fn generate_sequence_steps(patterns: &[ModelPattern], kws: &HashSet<String>) -> Result<TokenStream> {
    patterns.iter()
        .map(|p| generate_pattern_step(p, kws))
        .collect::<Result<TokenStream>>()
}

fn get_simple_peek(pattern: &ModelPattern, kws: &HashSet<String>) -> Result<Option<TokenStream>> {
    match pattern {
        ModelPattern::Lit(lit) => {
            let token_type = resolve_token_type(lit, kws)?;
            Ok(Some(quote!(#token_type)))
        },
        ModelPattern::Bracketed(_) => Ok(Some(quote!(syn::token::Bracket))),
        ModelPattern::Braced(_) => Ok(Some(quote!(syn::token::Brace))),
        ModelPattern::Parenthesized(_) => Ok(Some(quote!(syn::token::Paren))),
        ModelPattern::Optional(inner) | ModelPattern::Repeat(inner) | ModelPattern::Plus(inner) => 
            get_simple_peek(inner, kws),
        _ => Ok(None)
    }
}

fn resolve_token_type(lit: &syn::LitStr, custom_keywords: &HashSet<String>) -> Result<syn::Type> {
    let s = lit.value();
    if custom_keywords.contains(&s) {
        let ident = format_ident!("{}", s);
        return Ok(parse_quote!(kw::#ident));
    }
    syn::parse_str::<syn::Type>(&format!("Token![{}]", s))
        .map_err(|_| syn::Error::new(lit.span(), format!("Invalid token literal: '{}'", s)))
}

fn collect_custom_keywords(grammar: &GrammarDefinition) -> HashSet<String> {
    let mut kws = HashSet::new();
    grammar.rules.iter()
        .flat_map(|r| &r.variants)
        .for_each(|v| collect_from_patterns(&v.pattern, &mut kws));
    kws
}

fn collect_from_patterns(patterns: &[ModelPattern], kws: &mut HashSet<String>) {
    for p in patterns {
        match p {
            ModelPattern::Lit(lit) => {
                let s = lit.value();
                if is_identifier(&s) && !is_rust_keyword(&s) { kws.insert(s); }
            },
            ModelPattern::Group(alts) => alts.iter().for_each(|alt| collect_from_patterns(alt, kws)),
            ModelPattern::Bracketed(s) | ModelPattern::Braced(s) | ModelPattern::Parenthesized(s) => 
                collect_from_patterns(s, kws),
            ModelPattern::Optional(i) | ModelPattern::Repeat(i) | ModelPattern::Plus(i) => 
                collect_from_patterns(std::slice::from_ref(i), kws),
            _ => {}
        }
    }
}

fn is_identifier(s: &str) -> bool {
    s.chars().next().map_or(false, |c| c.is_alphabetic() || c == '_') && 
    s.chars().all(|c| c.is_alphanumeric() || c == '_')
}

fn is_rust_keyword(s: &str) -> bool {
    matches!(s, "fn" | "let" | "struct" | "enum" | "if" | "else" | "while" | "loop" | "for" | "match" | "return" | "pub" | "mod" | "use" | "type" | "trait" | "impl" | "const" | "static" | "mut" | "unsafe" | "extern" | "ref" | "self" | "Self" | "super" | "crate" | "async" | "await" | "where" | "move" | "true" | "false" | "in" | "as" | "dyn")
}

fn is_builtin(name: &syn::Ident) -> bool {
    matches!(name.to_string().as_str(), "ident" | "int_lit" | "string_lit")
}

fn map_builtin(name: &syn::Ident) -> TokenStream {
    match name.to_string().as_str() {
        "ident" => quote! { rt::parse_ident(input)? },
        "int_lit" => quote! { rt::parse_int::<i32>(input)? },
        "string_lit" => quote! { input.parse::<syn::LitStr>()?.value() },
        _ => unreachable!(),
    }
}
