use crate::model::*;
use super::{analysis, pattern};
use std::collections::{HashMap, HashSet};
use syn::Result;
use quote::{quote, format_ident};
use proc_macro2::TokenStream;

pub fn generate_rule(rule: &Rule, custom_keywords: &HashSet<String>) -> Result<TokenStream> {
    let name = &rule.name;
    let fn_name = format_ident!("parse_{}", name);
    let ret_type = &rule.return_type;
    
    let is_public = rule.is_pub || name == "main";
    let vis = if is_public { quote!(pub) } else { quote!() };
    
    let body = generate_variants_internal(&rule.variants, true, custom_keywords)?; 

    Ok(quote! {
        #vis fn #fn_name(input: ParseStream) -> Result<#ret_type> {
            #body
        }
    })
}

// Public für pattern.rs (für Groups), aber sonst intern
pub fn generate_variants_internal(
    variants: &[RuleVariant], 
    is_top_level: bool,
    custom_keywords: &HashSet<String>
) -> Result<TokenStream> {
    if variants.is_empty() {
        return Ok(quote! { Err(input.error("No variants defined")) });
    }

    // 1. Analyse Phase: Zähle Start-Tokens
    let mut token_counts = HashMap::new();
    for v in variants {
        if let Some(token_str) = analysis::get_peek_token_string(&v.pattern) {
            *token_counts.entry(token_str).or_insert(0) += 1;
        }
    }

    let arms = variants.iter().map(|variant| {
        let logic = pattern::generate_sequence(&variant.pattern, &variant.action, custom_keywords)?;
        
        let peek_token_obj = variant.pattern.first()
            .and_then(|f| analysis::get_simple_peek(f, custom_keywords).ok().flatten());
        
        let peek_str = analysis::get_peek_token_string(&variant.pattern);

        match (peek_token_obj, peek_str) {
            (Some(token_code), Some(token_key)) => {
                let count = token_counts.get(&token_key).unwrap_or(&0);
                
                if *count == 1 {
                    // UNIQUE PREFIX -> COMMIT (Kein attempt)
                    Ok(quote! {
                        if input.peek(#token_code) {
                            #logic
                        }
                    })
                } else {
                    // AMBIGUOUS PREFIX -> ATTEMPT (Backtracking nötig)
                    Ok(quote! {
                        if input.peek(#token_code) {
                            if let Some(res) = rt::attempt(input, |input| { #logic })? {
                                Ok(res)
                            } else {
                                None
                            }
                        }
                    })
                }
            },
            _ => {
                // BLIND START -> ATTEMPT
                Ok(quote! { 
                    if let Some(res) = rt::attempt(input, |input| { #logic })? { 
                        Ok(res) 
                    } 
                })
            }
        }
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

