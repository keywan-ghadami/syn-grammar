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
    
    // Check for direct left recursion
    let (recursive, base) = split_left_recursive(name, &rule.variants);

    let body = if recursive.is_empty() {
        generate_variants_internal(&rule.variants, true, custom_keywords)?
    } else {
        if base.is_empty() {
            return Err(syn::Error::new(name.span(), "Left-recursive rule requires at least one non-recursive base variant."));
        }

        let base_logic = generate_variants_internal(&base, true, custom_keywords)?;
        let loop_logic = generate_recursive_loop_body(&recursive, custom_keywords)?;

        quote! {
            let mut lhs = #base_logic;
            loop {
                #loop_logic
                break;
            }
            Ok(lhs)
        }
    };

    Ok(quote! {
        #vis fn #fn_name(input: ParseStream) -> Result<#ret_type> {
            #body
        }
    })
}

fn split_left_recursive(rule_name: &syn::Ident, variants: &[RuleVariant]) -> (Vec<RuleVariant>, Vec<RuleVariant>) {
    let mut recursive = Vec::new();
    let mut base = Vec::new();

    for v in variants {
        if let Some(ModelPattern::RuleCall { rule_name: r, .. }) = v.pattern.first() {
            if r == rule_name {
                recursive.push(v.clone());
                continue;
            }
        }
        base.push(v.clone());
    }
    (recursive, base)
}

fn generate_recursive_loop_body(variants: &[RuleVariant], kws: &HashSet<String>) -> Result<TokenStream> {
    let arms = variants.iter().map(|variant| {
        // Pattern without the first element (the left-recursive call)
        let tail_pattern = &variant.pattern[1..];
        
        // Binding for the LHS (e.g. "l" in "l:expr + ...")
        let lhs_binding = match &variant.pattern[0] {
            ModelPattern::RuleCall { binding: Some(b), .. } => Some(b),
            _ => None
        };

        let bind_stmt = if let Some(b) = lhs_binding {
            quote! { let #b = lhs; }
        } else {
            quote! {}
        };

        let logic = pattern::generate_sequence(tail_pattern, &variant.action, kws)?;
        
        // Peek logic on the *first token of the tail*
        let peek_token_obj = tail_pattern.first()
            .and_then(|f| analysis::get_simple_peek(f, kws).ok().flatten());
        
        match peek_token_obj {
            Some(token_code) => {
                Ok(quote! {
                    if input.peek(#token_code) {
                        // Speculative attempt for the tail
                        if let Some(new_val) = rt::attempt(input, |input| { 
                            #bind_stmt
                            #logic 
                        })? {
                            lhs = new_val;
                            continue;
                        }
                    }
                })
            },
            None => {
                // Blind attempt
                Ok(quote! {
                    if let Some(new_val) = rt::attempt(input, |input| { 
                        #bind_stmt
                        #logic 
                    })? {
                        lhs = new_val;
                        continue;
                    }
                })
            }
        }
    }).collect::<Result<Vec<_>>>()?;

    Ok(quote! { #(#arms)* })
}

pub fn generate_variants_internal(
    variants: &[RuleVariant], 
    is_top_level: bool,
    _custom_keywords: &HashSet<String> // Wird hier aktuell nicht mehr direkt gebraucht
) -> Result<TokenStream> {
    if variants.is_empty() {
        return Ok(quote! { Err(input.error("No variants defined")) });
    }

    // 1. Analyse Phase: Zähle Start-Tokens
    let mut token_counts = HashMap::new();
    for v in variants {
        // Hinweis: Hier wurde 'custom_keywords' aus dem Aufruf entfernt (wie im vorherigen Fix besprochen)
        if let Some(token_str) = analysis::get_peek_token_string(&v.pattern) {
            *token_counts.entry(token_str).or_insert(0) += 1;
        }
    }

    let arms = variants.iter().map(|variant| {
        // Wir übergeben weiterhin custom_keywords für die Code-Generierung der Sequenz
        let logic = pattern::generate_sequence(&variant.pattern, &variant.action, _custom_keywords)?;
        
        // Peek-Token holen (braucht noch kws für Typ-Auflösung)
        let peek_token_obj = variant.pattern.first()
            .and_then(|f| analysis::get_simple_peek(f, _custom_keywords).ok().flatten());
        
        let peek_str = analysis::get_peek_token_string(&variant.pattern);

        match (peek_token_obj, peek_str) {
            (Some(token_code), Some(token_key)) => {
                let count = token_counts.get(&token_key).unwrap_or(&0);
                
                if *count == 1 {
                    // UNIQUE PREFIX -> COMMIT
                    // Strategie: Wenn Token passt, MUSS es diese Regel sein.
                    // Wir führen aus und returnen das Ergebnis sofort.
                    // Fehler hier brechen die Funktion ab (gewollt).
                    Ok(quote! {
                        if input.peek(#token_code) {
                            return #logic;
                        }
                    })
                } else {
                    // AMBIGUOUS PREFIX -> ATTEMPT (Backtracking)
                    // Strategie: Wenn Token passt, probieren wir es "sandbox"-mäßig.
                    // Wenn es klappt -> return Ok.
                    // Wenn nicht (None) -> machen wir NICHTS und der Code läuft weiter zur nächsten Variante.
                    Ok(quote! {
                        if input.peek(#token_code) {
                            if let Some(res) = rt::attempt(input, |input| { #logic })? {
                                return Ok(res);
                            }
                        }
                    })
                }
            },
            _ => {
                // BLIND START -> ATTEMPT
                // Kein Peek möglich, wir müssen es probieren.
                Ok(quote! { 
                    if let Some(res) = rt::attempt(input, |input| { #logic })? { 
                        return Ok(res); 
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

    // ÄNDERUNG: Statt #(#arms else)* nutzen wir eine flache Liste.
    // Da jeder Block mit 'return' endet (bei Erfolg), wirkt das wie ein "First Match Wins".
    // Wenn nichts matcht, fallen wir unten durch in den Error.
    Ok(quote! {
        #(#arms)*
        Err(input.error(#error_msg))
    })
}
