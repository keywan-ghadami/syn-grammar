use syn_grammar_model::{model::*, analysis};
use super::pattern;
use std::collections::{HashMap, HashSet};
use syn::Result;
use quote::{quote, format_ident};
use proc_macro2::TokenStream;

pub fn generate_rule(rule: &Rule, custom_keywords: &HashSet<String>) -> Result<TokenStream> {
    let name = &rule.name;
    let fn_name = format_ident!("parse_{}", name);
    let ret_type = &rule.return_type;
    
    let doc_comment = format!("Parser for rule `{}`.", name);

    let params = rule.params.iter().map(|(name, ty)| {
        quote! { , #name : #ty }
    });

    let is_public = rule.is_pub || name == "main";
    let vis = if is_public { quote!(pub) } else { quote!() };
    
    // Check for direct left recursion
    let (recursive_refs, base_refs) = analysis::split_left_recursive(name, &rule.variants);

    let body = if recursive_refs.is_empty() {
        generate_variants_internal(&rule.variants, true, custom_keywords)?
    } else {
        if base_refs.is_empty() {
            return Err(syn::Error::new(name.span(), "Left-recursive rule requires at least one non-recursive base variant."));
        }

        // Clone variants back to owned structs for the generator
        let base_owned: Vec<RuleVariant> = base_refs.into_iter().cloned().collect();
        let recursive_owned: Vec<RuleVariant> = recursive_refs.into_iter().cloned().collect();

        let base_logic = generate_variants_internal(&base_owned, true, custom_keywords)?;
        let loop_logic = generate_recursive_loop_body(&recursive_owned, custom_keywords)?;

        quote! {
            let mut lhs = {
                let base_parser = |input: ParseStream| -> Result<#ret_type> {
                    #base_logic
                };
                base_parser(input)?
            };
            loop {
                #loop_logic
                break;
            }
            Ok(lhs)
        }
    };

    Ok(quote! {
        #[doc = #doc_comment]
        #vis fn #fn_name(input: ParseStream #(#params)*) -> Result<#ret_type> {
            #body
        }
    })
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
            quote! { let #b = lhs.clone(); }
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
                        // SAFETY: Capture cursor to check for progress
                        let _start_cursor = input.cursor();

                        // Speculative attempt for the tail
                        if let Some(new_val) = rt::attempt(input, |input| { 
                            #bind_stmt
                            #logic 
                        })? {
                            // SAFETY: Prevent infinite loop on nullable tails
                            if _start_cursor == input.cursor() {
                                return Err(input.error("Left-recursive rule matched empty string (infinite loop detected)"));
                            }
                            lhs = new_val;
                            continue;
                        }
                    }
                })
            },
            None => {
                // Blind attempt
                Ok(quote! {
                    // SAFETY: Capture cursor to check for progress
                    let _start_cursor = input.cursor();

                    if let Some(new_val) = rt::attempt(input, |input| { 
                        #bind_stmt
                        #logic 
                    })? {
                        // SAFETY: Prevent infinite loop on nullable tails
                        if _start_cursor == input.cursor() {
                            return Err(input.error("Left-recursive rule matched empty string (infinite loop detected)"));
                        }
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
    _custom_keywords: &HashSet<String> // Currently not directly used here
) -> Result<TokenStream> {
    if variants.is_empty() {
        return Ok(quote! { Err(input.error("No variants defined")) });
    }

    // 1. Analysis Phase: Count start tokens
    let mut token_counts = HashMap::new();
    for v in variants {
        if let Some(token_str) = analysis::get_peek_token_string(&v.pattern) {
            *token_counts.entry(token_str).or_insert(0) += 1;
        }
    }

    let arms = variants.iter().map(|variant| {
        // Check for Cut Operator
        let cut_info = analysis::find_cut(&variant.pattern);
        
        // Get Peek Token
        let peek_token_obj = variant.pattern.first()
            .and_then(|f| analysis::get_simple_peek(f, _custom_keywords).ok().flatten());
        let peek_str = analysis::get_peek_token_string(&variant.pattern);
        
        // Determine if we have a unique prefix (optimization)
        let is_unique = if let (_, Some(token_key)) = (&peek_token_obj, &peek_str) {
            token_counts.get(token_key).map(|c| *c == 1).unwrap_or(false)
        } else {
            false
        };

        if let Some(cut) = cut_info {
            // --- CUT LOGIC (A => B) ---
            let pre_cut = cut.pre_cut;
            let post_cut = cut.post_cut;
            
            let pre_bindings = analysis::collect_bindings(pre_cut);
            let pre_logic = pattern::generate_sequence_steps(pre_cut, _custom_keywords)?;
            let post_logic = pattern::generate_sequence_steps(post_cut, _custom_keywords)?;
            let action = &variant.action;

            // Construct the logic block
            let logic_block = if is_unique {
                // If unique, we don't need speculative parsing for the pre-cut part either.
                // Just run everything linearly.
                quote! {
                    {
                        let run = || -> syn::Result<_> {
                            #pre_logic
                            #post_logic
                            Ok(#action)
                        };
                        match run() {
                            Ok(v) => return Ok(v),
                            Err(e) => {
                                rt::set_fatal(true);
                                return Err(e);
                            }
                        }
                    }
                }
            } else {
                // Ambiguous: Speculative Pre-Cut, Fatal Post-Cut
                quote! {
                    // 1. Speculative Phase (Pre-Cut)
                    let pre_result = rt::attempt(input, |input| {
                        #pre_logic
                        Ok(( #(#pre_bindings),* ))
                    })?;

                    if let Some(( #(#pre_bindings),* )) = pre_result {
                        // 2. Commit Phase (Post-Cut) - Errors here are fatal!
                        let post_run = || -> syn::Result<_> {
                            #post_logic
                            Ok(#action)
                        };
                        match post_run() {
                            Ok(v) => return Ok(v),
                            Err(e) => {
                                rt::set_fatal(true);
                                return Err(e);
                            }
                        }
                    }
                }
            };

            // Wrap with Peek check if available
            if let Some(token_code) = peek_token_obj {
                Ok(quote! {
                    if input.peek(#token_code) {
                        #logic_block
                    }
                })
            } else {
                Ok(logic_block)
            }

        } else {
            // --- STANDARD LOGIC (No Cut) ---
            let logic = pattern::generate_sequence(&variant.pattern, &variant.action, _custom_keywords)?;

            if is_unique {
                // Unique Prefix -> Commit immediately
                let token_code = peek_token_obj.as_ref().unwrap();
                Ok(quote! {
                    if input.peek(#token_code) {
                        let run = || -> syn::Result<_> {
                            #logic
                        };
                        match run() {
                            Ok(v) => return Ok(v),
                            Err(e) => {
                                rt::set_fatal(true);
                                return Err(e);
                            }
                        }
                    }
                })
            } else if let Some(token_code) = peek_token_obj {
                // Ambiguous Prefix -> Attempt
                Ok(quote! {
                    if input.peek(#token_code) {
                        if let Some(res) = rt::attempt(input, |input| { #logic })? {
                            return Ok(res);
                        }
                    }
                })
            } else {
                // Blind -> Attempt
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

    // CHANGE: Instead of #(#arms else)* we use a flat list.
    // Since every block ends with 'return' (on success), this acts like "First Match Wins".
    // If nothing matches, we fall through to the error.
    Ok(quote! {
        #(#arms)*
        
        if let Some(best_err) = rt::take_best_error() {
            Err(best_err)
        } else {
            Err(input.error(#error_msg))
        }
    })
}
