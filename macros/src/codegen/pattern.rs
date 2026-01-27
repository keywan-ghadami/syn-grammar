use crate::model::*;
use super::analysis; 
use std::collections::HashSet;
use syn::{Result};
use quote::{quote, quote_spanned, format_ident};
use proc_macro2::TokenStream;

pub fn generate_sequence(patterns: &[ModelPattern], action: &TokenStream, kws: &HashSet<String>) -> Result<TokenStream> {
    let steps = generate_sequence_steps(patterns, kws)?;
    Ok(quote! { { #steps Ok(#action) } })
}

pub fn generate_sequence_steps(patterns: &[ModelPattern], kws: &HashSet<String>) -> Result<TokenStream> {
    let steps = patterns.iter()
        .map(|p| generate_pattern_step(p, kws))
        .collect::<Result<Vec<_>>>()?;
    Ok(quote! { #(#steps)* })
}

fn generate_pattern_step(pattern: &ModelPattern, kws: &HashSet<String>) -> Result<TokenStream> {
    let span = pattern.span();

    match pattern {
        ModelPattern::Cut => {
            // The Cut operator is handled at the RuleVariant level (in rule.rs).
            // If it appears here, it's likely inside a group or handled implicitly.
            // We emit no code for it in the sequence flow.
            Ok(quote!())
        },
        ModelPattern::Lit(lit) => {
            let token_type = analysis::resolve_token_type(lit, kws)?;
            Ok(quote_spanned! {span=> let _ = input.parse::<#token_type>()?; })
        },
        ModelPattern::RuleCall { binding, rule_name, args } => {
            let func_call = generate_rule_call_expr(rule_name, args);
            Ok(if let Some(bind) = binding {
                quote_spanned! {span=> let #bind = #func_call; }
            } else {
                quote_spanned! {span=> let _ = #func_call; }
            })
        },
        
        ModelPattern::Repeat(inner) => {
            if let ModelPattern::RuleCall { binding: Some(bind), rule_name, args } = &**inner {
                 let func_call = generate_rule_call_expr(rule_name, args);
                 let peek_check = if let Some(peek) = analysis::get_simple_peek(inner, kws)? {
                     quote!(input.peek(#peek))
                 } else {
                     quote!(true)
                 };

                 if analysis::get_simple_peek(inner, kws)?.is_some() {
                     Ok(quote_spanned! {span=> 
                        let mut #bind = Vec::new();
                        while #peek_check {
                            let val = #func_call;
                            #bind.push(val);
                        }
                     })
                 } else {
                     Ok(quote_spanned! {span=> 
                        let mut #bind = Vec::new();
                        while let Some(val) = rt::attempt(input, |input| { Ok(#func_call) })? {
                            #bind.push(val);
                        }
                     })
                 }
            } else {
                let inner_logic = generate_pattern_step(inner, kws)?;
                Ok(quote_spanned! {span=> while let Some(_) = rt::attempt(input, |input| { #inner_logic Ok(()) })? {} })
            }
        },
        
        ModelPattern::Plus(inner) => {
             if let ModelPattern::RuleCall { binding: Some(bind), rule_name, args } = &**inner {
                 let func_call = generate_rule_call_expr(rule_name, args);
                 let peek_check = if let Some(peek) = analysis::get_simple_peek(inner, kws)? {
                     quote!(input.peek(#peek))
                 } else {
                     quote!(true)
                 };
                 
                 if analysis::get_simple_peek(inner, kws)?.is_some() {
                     Ok(quote_spanned! {span=> 
                        let mut #bind = Vec::new();
                        #bind.push(#func_call);
                        while #peek_check {
                            #bind.push(#func_call);
                        }
                     })
                 } else {
                      Ok(quote_spanned! {span=> 
                        let mut #bind = Vec::new();
                        #bind.push(#func_call);
                        while let Some(val) = rt::attempt(input, |input| { Ok(#func_call) })? {
                            #bind.push(val);
                        }
                     })
                 }
             } else {
                let inner_logic = generate_pattern_step(inner, kws)?;
                Ok(quote_spanned! {span=> 
                    #inner_logic
                    while let Some(_) = rt::attempt(input, |input| { #inner_logic Ok(()) })? {}
                })
             }
        },

        ModelPattern::Optional(inner) => {
            let inner_logic = generate_pattern_step(inner, kws)?;
            Ok(quote_spanned! {span=> let _ = rt::attempt(input, |input| { #inner_logic Ok(()) })?; })
        },
        ModelPattern::Group(alts) => {
            use super::rule::generate_variants_internal;
            let temp_variants = alts.iter()
                .map(|pat_seq| RuleVariant { pattern: pat_seq.clone(), action: quote!({}) })
                .collect::<Vec<_>>();
            let variant_logic = generate_variants_internal(&temp_variants, false, kws)?;
            Ok(quote_spanned! {span=> { #variant_logic }?; })
        },

        ModelPattern::Bracketed(s) | ModelPattern::Braced(s) | ModelPattern::Parenthesized(s) => {
            let macro_name = match pattern {
                ModelPattern::Bracketed(_) => quote!(bracketed),
                ModelPattern::Braced(_) => quote!(braced),
                _ => quote!(parenthesized),
            };
            
            let inner_logic = generate_sequence_steps(s, kws)?;
            let bindings = analysis::collect_bindings(s);

            if bindings.is_empty() {
                Ok(quote_spanned! {span=> {
                    let content;
                    // FIX: Das '?' wurde entfernt, da das Makro selbst returned
                    let _ = syn::#macro_name!(content in input);
                    let input = &content;
                    #inner_logic
                }})
            } else if bindings.len() == 1 {
                let bind = &bindings[0];
                Ok(quote_spanned! {span=> 
                    let #bind = {
                        let content;
                        let _ = syn::#macro_name!(content in input);
                        let input = &content;
                        #inner_logic
                        #bind
                    };
                })
            } else {
                Ok(quote_spanned! {span=> 
                    let (#(#bindings),*) = {
                        let content;
                        let _ = syn::#macro_name!(content in input);
                        let input = &content;
                        #inner_logic
                        (#(#bindings),*)
                    };
                })
            }
        },
    }
}

fn generate_rule_call_expr(rule_name: &syn::Ident, args: &[syn::Lit]) -> TokenStream {
    if is_builtin(rule_name) {
        map_builtin(rule_name)
    } else {
        let f = format_ident!("parse_{}", rule_name);
        if args.is_empty() { quote!(#f(input)?) } else { quote!(#f(input, #(#args),*)?) }
    }
}

fn is_builtin(name: &syn::Ident) -> bool {
    matches!(name.to_string().as_str(), "ident" | "int_lit" | "string_lit" | "rust_type" | "rust_block" | "lit_str")
}

fn map_builtin(name: &syn::Ident) -> TokenStream {
    match name.to_string().as_str() {
        "ident" => quote! { rt::parse_ident(input)? },
        "int_lit" => quote! { rt::parse_int::<i32>(input)? },
        "string_lit" => quote! { input.parse::<syn::LitStr>()?.value() },
        "lit_str" => quote! { input.parse::<syn::LitStr>()? },
        "rust_type" => quote! { input.parse::<syn::Type>()? },
        "rust_block" => quote! { input.parse::<syn::Block>()? },
        _ => unreachable!(),
    }
}
