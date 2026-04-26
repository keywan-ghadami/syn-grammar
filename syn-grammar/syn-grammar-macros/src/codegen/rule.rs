use super::pattern;
use super::CodegenContext;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::collections::HashMap;
use syn::Result;
use syn_grammar_model::{analysis, model::*};

pub fn generate_rule(rule: &Rule, ctx: &CodegenContext) -> Result<TokenStream> {
    let name = &rule.name;

    // 1. Schlangennamen in lesbaren Semantik-String übersetzen
    let context_name = name.to_string().replace("_", " ");

    let fn_name = format_ident!("parse_{}", name);
    let impl_name = format_ident!("parse_{}_impl", name);
    let ret_type = &rule.return_type;
    let attrs = &rule.attrs;
    let generics = &rule.generics; // Include where clause if present

    // Filter attributes for the implementation function
    let impl_attrs: Vec<&syn::Attribute> = attrs
        .iter()
        .filter(|a| {
            let p = a.path();
            p.is_ident("cfg")
                || p.is_ident("cfg_attr")
                || p.is_ident("allow")
                || p.is_ident("warn")
                || p.is_ident("deny")
                || p.is_ident("forbid")
        })
        .collect();

    // Default doc comment if none provided
    let default_doc = if attrs.iter().any(|a| a.path().is_ident("doc")) {
        quote!()
    } else {
        let msg = format!("Parser for rule `{}`.", name);
        quote!(#[doc = #msg])
    };

    let params: Vec<_> = rule
        .params
        .iter()
        .filter_map(|p| {
            p.ty.as_ref().map(|t| {
                let name = &p.name;
                quote! { , #name : #t }
            })
        })
        .collect();

    // Params for the impl call (forwarding arguments)
    let param_names: Vec<_> = rule
        .params
        .iter()
        .filter_map(|p| {
            p.ty.as_ref().map(|_| {
                let name = &p.name;
                quote! { , #name }
            })
        })
        .collect();

    let is_public = rule.is_pub || name == "main";
    let vis = if is_public { quote!(pub) } else { quote!() };

    // Check for direct left recursion
    let (recursive_refs, base_refs) = analysis::split_left_recursive(name, &rule.variants);

    // Add where clause from generics
    let where_clause = &generics.where_clause;

    // Check if lexical
    let lexical_block_start = if rule.is_lexical {
        quote! { ctx.enter_lexical(); }
    } else {
        quote! {}
    };

    let lexical_block_end = if rule.is_lexical {
        quote! { ctx.exit_mode(); }
    } else {
        quote! {}
    };

    let body = if !recursive_refs.is_empty() {
        if base_refs.is_empty() {
            return Err(syn::Error::new(
                name.span(),
                "Left-recursive rule requires at least one non-recursive base variant.",
            ));
        }

        let base_owned: Vec<RuleVariant> = base_refs.into_iter().cloned().collect();
        let recursive_owned: Vec<RuleVariant> = recursive_refs.into_iter().cloned().collect();

        let base_logic = generate_variants_internal(&base_owned, true, ctx)?;
        let loop_logic = generate_recursive_loop_body(&recursive_owned, ctx)?;

        quote! {
            let mut lhs = {
                let base_parser = |mut input: ParseStream, ctx: &mut rt::ParseContext| -> Result<#ret_type> {
                    #base_logic
                };
                base_parser(input, ctx)?
            };
            loop {
                #loop_logic
                break;
            }
            Ok(lhs)
        }
    } else {
        generate_variants_internal(&rule.variants, true, ctx)?
    };
    
    let rule_logic = body;

    Ok(quote! {
        #(#attrs)*
        #default_doc
        #vis fn #fn_name(input: ParseStream #(#params)*) -> Result<#ret_type> #where_clause {
            let mut ctx = rt::ParseContext::new();
            match #impl_name(input, &mut ctx #(#param_names)*) {
                Ok(val) => {
                    // CRITICAL FIX: Wenn der Parser erfolgreich abschließt, aber Tokens 
                    // übrig lässt, extrahieren wir den präzisen Abbruchgrund aus dem Kontext,
                    // anstatt auf den generischen "unexpected token" Fehler von syn zu warten.
                    if !input.is_empty() {
                        if let Some(best) = ctx.take_best_error() {
                            return Err(best);
                        }
                    }
                    Ok(val)
                },
                Err(e) => {
                    if let Some(best) = ctx.take_best_error() {
                        Err(best)
                    } else {
                        Err(e)
                    }
                }
            }
        }

        #[doc(hidden)]
        #(#impl_attrs)*
        pub fn #impl_name(mut input: syn::parse::ParseStream, ctx: &mut rt::ParseContext #(#params)*) -> syn::Result<#ret_type> #where_clause {
            
            // 2. Den übersetzten Kontext-String statt des harten Bezeichners übergeben
            ctx.enter_rule(#context_name); 
            
            #lexical_block_start
            let _start_span = input.span();

            let _res = (|| -> syn::Result<_> {
                #rule_logic
            })();

            #lexical_block_end
            
            ctx.exit_rule(); // Stack bereinigen
            
            _res
        }
    })
}

fn generate_recursive_loop_body(
    variants: &[RuleVariant],
    ctx: &CodegenContext,
) -> Result<TokenStream> {
    let arms = variants.iter().map(|variant| {
        let tail_pattern = &variant.pattern[1..];

        let lhs_binding = match &variant.pattern[0] {
            ModelPattern::RuleCall { binding: Some(b), .. } => Some(b),
            _ => None
        };

        let bind_stmt = if let Some(b) = lhs_binding {
            quote! { let #b = lhs.clone(); }
        } else {
            quote! {}
        };

        let logic = pattern::generate_sequence(tail_pattern, &variant.action, ctx)?;

        let peek_token_obj = tail_pattern.first()
            .and_then(|f| analysis::get_simple_peek(f, ctx.custom_keywords).ok().flatten());

        let label_lit = if let Some(l) = &variant.label {
            quote!(Some(#l))
        } else {
            quote!(None)
        };

        match peek_token_obj {
            Some(token_code) => {
                Ok(quote! {
                    if input.peek(#token_code) {
                        let _start_cursor = input.cursor();
                        // Pass ctx to attempt_labeled
                        if let Some(new_val) = rt::attempt_labeled(input, ctx, #label_lit, |mut input, ctx| {
                            #bind_stmt
                            #logic
                        })? {
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
                Ok(quote! {
                    let _start_cursor = input.cursor();
                    // Pass ctx to attempt_labeled
                    if let Some(new_val) = rt::attempt_labeled(input, ctx, #label_lit, |mut input, ctx| {
                        #bind_stmt
                        #logic
                    })? {
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
    ctx: &CodegenContext,
) -> Result<TokenStream> {
    if variants.is_empty() {
        return Ok(quote! { Err(input.error("No variants defined")) });
    }

    let mut token_counts = HashMap::new();
    for v in variants {
        let is_nullable = v.pattern.first().is_none_or(analysis::is_nullable);
        if !is_nullable {
            if let Some(token_str) = analysis::get_peek_token_string(&v.pattern) {
                *token_counts.entry(token_str).or_insert(0) += 1;
            }
        }
    }

    let arms = variants
        .iter()
        .map(|variant| {
            // Label determination
            let label_str = if let Some(l) = &variant.label {
                 Some(l.clone())
             } else {
                // Try to derive a label from the first token if possible
                analysis::get_peek_token_string(&variant.pattern)
             };

             let label_lit = if let Some(l) = &label_str {
                 quote!(Some(#l))
             } else {
                 quote!(None)
             };

            let failure_rec = if let Some(l) = label_str {
                 quote! {
                     if !ctx.stop_aggregation(input.span()) {
                         _shallow_failures.push(#l);
                     }
                 }
             } else {
                 quote! {}
             };

            let cut_info = analysis::find_cut(&variant.pattern);
            let first_pat = variant.pattern.first();
            let is_nullable = first_pat.is_none_or(analysis::is_nullable);

            let peek_token_obj = if !is_nullable {
                first_pat.and_then(|f| {
                    analysis::get_simple_peek(f, ctx.custom_keywords)
                        .ok()
                        .flatten()
                })
            } else {
                None
            };

            let peek_str = if !is_nullable {
                analysis::get_peek_token_string(&variant.pattern)
            } else {
                None
            };

            let is_unique = if let (_, Some(token_key)) = (&peek_token_obj, &peek_str) {
                token_counts
                    .get(token_key)
                    .map(|c| *c == 1)
                    .unwrap_or(false)
            } else {
                false
            };

            let logic = if let Some(cut) = cut_info {
                let pre_cut = cut.pre_cut;
                let post_cut = cut.post_cut;

                let pre_bindings = analysis::collect_bindings(pre_cut);
                let pre_logic = pattern::generate_sequence_steps(pre_cut, ctx)?;
                let post_logic = pattern::generate_sequence_steps(post_cut, ctx)?;
                let action = &variant.action;

                // For cut, we use attempt_labeled for the pre-part.
                let logic_block = if is_unique {
                     quote! {
                        {
                            // Unique + Cut.
                            let pre_res = rt::attempt_labeled(input, ctx, #label_lit, |mut input, ctx| {
                                #pre_logic
                                Ok(( #(#pre_bindings),* ))
                            })?;

                            match pre_res {
                                Some(( #(#pre_bindings),* )) => {
                                    // Pre succeeded. Now run Post.
                                    let mut post_run = || -> syn::Result<_> {
                                        #post_logic
                                        let _semantic_res = (|| -> syn::Result<_> {
                                            Ok({ #action })
                                        })();
                                        match _semantic_res {
                                            Ok(_v) => Ok(_v),
                                            Err(_e) => {
                                                ctx.set_priority(rt::ParseContext::PRIO_STRUCTURAL);
                                                Err(_e)
                                            }
                                        }
                                    };
                                    match post_run() {
                                        Ok(v) => return Ok(v),
                                        Err(e) => {
                                            ctx.commit();
                                            return Err(e);
                                        }
                                    }
                                }
                                None => {
                                    // Pre failed. Since is_unique, this is FATAL.
                                    ctx.commit();
                                    return Err(input.error("propagating fatal unique cut error"));
                                }
                            }
                        }
                    }
                } else {
                    quote! {
                        // Pass ctx to attempt_labeled
                        let pre_result = rt::attempt_labeled(input, ctx, #label_lit, |mut input, ctx| {
                            #pre_logic
                            Ok(( #(#pre_bindings),* ))
                        })?;

                        if let Some(( #(#pre_bindings),* )) = pre_result {
                            let mut post_run = || -> syn::Result<_> {
                                #post_logic
                                let _semantic_res = (|| -> syn::Result<_> {
                                    Ok({ #action })
                                        })();
                                        match _semantic_res {
                                            Ok(_v) => Ok(_v),
                                            Err(_e) => {
                                                ctx.set_priority(rt::ParseContext::PRIO_STRUCTURAL);
                                                Err(_e)
                                            }
                                        }
                            };
                            match post_run() {
                                Ok(v) => return Ok(v),
                                Err(e) => {
                                    ctx.commit();
                                    return Err(e);
                                }
                            }
                        }
                    }
                };

                if let Some(token_code) = peek_token_obj {
                     quote! {
                        if input.peek(#token_code) {
                            #logic_block
                        }
                    }
                } else {
                    logic_block
                }
            } else {
                let logic = pattern::generate_sequence(
                    &variant.pattern,
                    &variant.action,
                    ctx,
                )?;

                if is_unique {
                    let token_code = peek_token_obj.as_ref().unwrap();
                     quote! {
                        if input.peek(#token_code) {
                            match rt::attempt_labeled(input, ctx, #label_lit, |mut input, ctx| {
                                #logic
                            })? {
                                Some(v) => return Ok(v),
                                None => {
                                    ctx.commit();
                                    return Err(input.error("propagating fatal unique error"));
                                }
                            }
                        }
                    }
                } else if let Some(token_code) = peek_token_obj {
                     quote! {
                        if input.peek(#token_code) {
                            // Pass ctx to attempt_labeled
                            if let Some(res) = rt::attempt_labeled(input, ctx, #label_lit, |mut input, ctx| { #logic })? {
                                return Ok(res);
                            }
                        }
                    }
                } else {
                     quote! {
                        // Pass ctx to attempt_labeled
                        if let Some(res) = rt::attempt_labeled(input, ctx, #label_lit, |mut input, ctx| { #logic })? {
                            return Ok(res);
                        }
                    }
                }
            };

            Ok(quote! {
                {
                    #logic
                }
                #failure_rec
            })
        })
        .collect::<Result<Vec<_>>>()?;

    let error_msg = if is_top_level {
        "No matching rule variant found"
    } else {
        "No matching variant in group"
    };

    Ok(quote! {
        let mut _shallow_failures = Vec::<&str>::new();
        let _start_span = input.span();

        #(#arms)*

        if ctx.stop_aggregation(_start_span) {
            // THE FIX: Do NOT destroy the best error by calling take_best_error here!
            // Just bubble up our internal dummy error which record_error will ignore.
            // This leaves the true significant error safely inside `ctx.best_error` 
            // until the top-level parse wrapper consumes it.
            return Err(input.error("__DUMMY_ERR_BUBBLE__"));
        }

        let mut error_to_return = if !_shallow_failures.is_empty() {
             _shallow_failures.sort();
             _shallow_failures.dedup();

             let msg = if _shallow_failures.len() == 1 {
                 format!("expected `{}`", _shallow_failures[0])
             } else {
                 let joined = _shallow_failures.iter()
                    .map(|s| format!("`{}`", s))
                    .collect::<Vec<_>>()
                    .join(", ");
                 format!("expected one of: {}", joined)
             };

             // CRITICAL FIX: Auch hier kein take_best_error()!
             // Dem Kontext die saubere Priorität für Aggregation mitteilen
             ctx.set_priority(rt::ParseContext::PRIO_AGGREGATED);
             input.error(msg)
        } else {
             // Fallback-Fehler, falls keine flachen Fehler vorliegen
             input.error(#error_msg)
        };

        if !input.is_empty() {
            if let Ok(tt) = input.fork().parse::<proc_macro2::TokenTree>() {
                let found = tt.to_string();
                if !found.trim().is_empty() {
                    let new_message = format!("{}; found unexpected token `{}`", error_to_return, found);
                    error_to_return = syn::Error::new(error_to_return.span(), new_message);
                }
            }
        }

        // NEW: Record aggregated error before returning, but ONLY if we have one.
        if !_shallow_failures.is_empty() {
            ctx.record_error(error_to_return.clone(), _start_span, None, rt::ParseContext::PRIO_AGGREGATED);
        }

        Err(error_to_return)
    })
}