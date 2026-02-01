use syn_grammar_model::{model::*, analysis};
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
        ModelPattern::Cut => Ok(quote!()),
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
                        // Pass ctx to attempt
                        while let Some(val) = rt::attempt(input, ctx, |input, ctx| { Ok(#func_call) })? {
                            #bind.push(val);
                        }
                     })
                 }
            } else {
                let inner_logic = generate_pattern_step(inner, kws)?;
                Ok(quote_spanned! {span=> 
                    // Pass ctx to attempt
                    while let Some(_) = rt::attempt(input, ctx, |input, ctx| { #inner_logic Ok(()) })? {} 
                })
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
                        // Pass ctx to attempt
                        while let Some(val) = rt::attempt(input, ctx, |input, ctx| { Ok(#func_call) })? {
                            #bind.push(val);
                        }
                     })
                 }
             } else {
                let inner_logic = generate_pattern_step(inner, kws)?;
                Ok(quote_spanned! {span=> 
                    #inner_logic
                    // Pass ctx to attempt
                    while let Some(_) = rt::attempt(input, ctx, |input, ctx| { #inner_logic Ok(()) })? {}
                })
             }
        },

        ModelPattern::Optional(inner) => {
            let inner_logic = generate_pattern_step(inner, kws)?;
            let peek_opt = analysis::get_simple_peek(inner, kws)?;
            let is_nullable = analysis::is_nullable(inner);

            if let (Some(peek), false) = (peek_opt, is_nullable) {
                Ok(quote_spanned! {span=> 
                    if input.peek(#peek) {
                        // Pass ctx to attempt
                        let _ = rt::attempt(input, ctx, |input, ctx| { #inner_logic Ok(()) })?; 
                    }
                })
            } else {
                Ok(quote_spanned! {span=> 
                    // Pass ctx to attempt
                    let _ = rt::attempt(input, ctx, |input, ctx| { #inner_logic Ok(()) })?; 
                })
            }
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

        ModelPattern::Recover { binding, body, sync } => {
            let effective_body = if let Some(bind) = binding {
                match &**body {
                    ModelPattern::RuleCall { binding: None, rule_name, args } => {
                        Box::new(ModelPattern::RuleCall { 
                            binding: Some(bind.clone()), 
                            rule_name: rule_name.clone(), 
                            args: args.clone() 
                        })
                    },
                    _ => return Err(syn::Error::new(span, "Binding on recover(...) is only supported if the body is a direct rule call."))
                }
            } else {
                body.clone()
            };

            let inner_logic = generate_pattern_step(&effective_body, kws)?;
            let sync_peek = analysis::get_simple_peek(sync, kws)?
                .ok_or_else(|| syn::Error::new(sync.span(), "Sync pattern in recover(...) must have a simple start token."))?;

            let bindings = analysis::collect_bindings(std::slice::from_ref(&effective_body));

            if bindings.is_empty() {
                Ok(quote_spanned! {span=> 
                    // Pass ctx to attempt_recover
                    if rt::attempt_recover(input, ctx, |input, ctx| { #inner_logic Ok(()) })?.is_none() {
                        rt::skip_until(input, |i| i.peek(#sync_peek))?;
                    }
                })
            } else {
                let none_exprs = bindings.iter().map(|_| quote!(Option::<_>::None));

                Ok(quote_spanned! {span=> 
                    // Pass ctx to attempt_recover
                    let (#(#bindings),*) = match rt::attempt_recover(input, ctx, |input, ctx| {
                        #inner_logic
                        Ok((#(#bindings),*))
                    })? {
                        Some(vals) => {
                            let (#(#bindings),*) = vals;
                            (#(Some(#bindings)),*)
                        },
                        None => {
                            rt::skip_until(input, |i| i.peek(#sync_peek))?;
                            (#(#none_exprs),*)
                        }
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
        // Call the _impl version and pass ctx
        let f = format_ident!("parse_{}_impl", rule_name);
        if args.is_empty() { 
            quote!(#f(input, ctx)?) 
        } else { 
            quote!(#f(input, ctx, #(#args),*)?) 
        }
    }
}

fn is_builtin(name: &syn::Ident) -> bool {
    matches!(name.to_string().as_str(), 
        "ident" | "integer" | "string" | "rust_type" | "rust_block" | "lit_str" |
        "lit_int" | "lit_char" | "lit_bool" | "lit_float" |
        "spanned_int_lit" | "spanned_string_lit" |
        "spanned_float_lit" | "spanned_bool_lit" | "spanned_char_lit"
    )
}

fn map_builtin(name: &syn::Ident) -> TokenStream {
    // Builtins are stateless, so they don't need ctx
    match name.to_string().as_str() {
        "ident" => quote! { rt::parse_ident(input)? },
        "integer" => quote! { rt::parse_int::<i32>(input)? },
        "string" => quote! { input.parse::<syn::LitStr>()?.value() },
        "lit_str" => quote! { input.parse::<syn::LitStr>()? },
        "rust_type" => quote! { input.parse::<syn::Type>()? },
        "rust_block" => quote! { input.parse::<syn::Block>()? },
        
        "lit_int" => quote! { input.parse::<syn::LitInt>()? },
        "lit_char" => quote! { input.parse::<syn::LitChar>()? },
        "lit_bool" => quote! { input.parse::<syn::LitBool>()? },
        "lit_float" => quote! { input.parse::<syn::LitFloat>()? },

        "spanned_int_lit" => quote! { 
            {
                let l = input.parse::<syn::LitInt>()?;
                (l.base10_parse::<i32>()?, l.span())
            }
        },
        "spanned_string_lit" => quote! { 
            {
                let l = input.parse::<syn::LitStr>()?;
                (l.value(), l.span())
            }
        },
        "spanned_float_lit" => quote! { 
            {
                let l = input.parse::<syn::LitFloat>()?;
                (l.base10_parse::<f64>()?, l.span())
            }
        },
        "spanned_bool_lit" => quote! { 
            {
                let l = input.parse::<syn::LitBool>()?;
                (l.value, l.span())
            }
        },
        "spanned_char_lit" => quote! { 
            {
                let l = input.parse::<syn::LitChar>()?;
                (l.value(), l.span())
            }
        },
        _ => unreachable!(),
    }
}
