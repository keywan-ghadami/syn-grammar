use crate::model::*;
use quote::{quote, quote_spanned, format_ident};
use proc_macro2::TokenStream;
use std::collections::HashSet;
use syn::{Result, parse_quote};
use itertools::Itertools; 

pub fn generate_rust(grammar: GrammarDefinition) -> Result<TokenStream> {
    let grammar_name = &grammar.name;
    let custom_keywords = collect_custom_keywords(&grammar);

    let kw_defs = (!custom_keywords.is_empty()).then(|| {
        let defs = custom_keywords.iter().map(|k| {
            let ident = format_ident!("{}", k);
            quote! { syn::custom_keyword!(#ident); }
        });
        quote! { pub mod kw { #(#defs)* } }
    });

    let inheritance = grammar.inherits.as_ref().map(|parent| {
        quote! { use super::#parent::*; }
    });

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
    let name = &rule.name;
    let fn_name = format_ident!("parse_{}", name);
    let ret_type = &rule.return_type;
    
    // FIX: Regel 'main' ist automatisch public
    let is_public = rule.is_pub || name == "main";
    let vis = if is_public { quote!(pub) } else { quote!() };
    
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
        
        let peek_token = variant.pattern.first()
            .and_then(|f| get_simple_peek(f, custom_keywords).ok().flatten());

        let attempt_block = quote! { rt::attempt(input, |input| { #logic })? };

        Ok(match peek_token {
            Some(token) => {
                quote! {
                    if let Some(res) = if input.peek(#token) { 
                        #attempt_block 
                    } else { 
                        None 
                    } { 
                        Ok(res) 
                    }
                }
            },
            None => {
                quote! { 
                    if let Some(res) = #attempt_block { 
                        Ok(res) 
                    } 
                }
            }
        })
    }).collect::<Result<Vec<_>>>()?;

    let error_msg = if is_top_level { 
        "No matching rule variant found" 
    } else { 
        "No matching variant in group" 
    };

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
            Ok(quote_spanned! {span=> let _ = rt::attempt(input, |input| { #inner_logic Ok(()) })?; })
        },
        ModelPattern::Repeat(inner) => {
            let inner_logic = generate_pattern_step(inner, kws)?;
            Ok(quote_spanned! {span=> while let Some(_) = rt::attempt(input, |input| { #inner_logic Ok(()) })? {} })
        },
        ModelPattern::Plus(inner) => {
            let inner_logic = generate_pattern_step(inner, kws)?;
            Ok(quote_spanned! {span=> 
                #inner_logic
                while let Some(_) = rt::attempt(input, |input| { #inner_logic Ok(()) })? {}
            })
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
    
    // 1. Check auf Custom Keywords
    if custom_keywords.contains(&s) {
        let ident = format_ident!("{}", s);
        return Ok(parse_quote!(kw::#ident));
    }

    // 2. Expliziter Check auf verbotene Klammern in Token![]
    // Klammern müssen über Pattern::Parenthesized etc. geparst werden!
    if matches!(s.as_str(), "(" | ")" | "[" | "]" | "{" | "}") {
        return Err(syn::Error::new(lit.span(), 
            format!("Invalid direct token literal: '{}'. Use paren(...), bracketed[...] or braced{{...}} instead.", s)));
    }

    // 3. Versuche, es als syn::Token zu parsen (z.B. Token![+])
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
    s.chars().next().is_some_and(|c| c.is_alphabetic() || c == '_') && 
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
