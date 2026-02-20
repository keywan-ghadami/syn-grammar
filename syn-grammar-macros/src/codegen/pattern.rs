use crate::backend::SynBackend;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::collections::HashSet;
use syn::{Lit, Result};
use syn_grammar_model::{analysis, model::*, Backend};

pub fn generate_sequence(
    patterns: &[ModelPattern],
    action: &TokenStream,
    kws: &HashSet<String>,
) -> Result<TokenStream> {
    let steps = generate_sequence_steps(patterns, kws)?;
    Ok(quote! { { #steps Ok({ #action }) } })
}

pub fn generate_sequence_steps(
    patterns: &[ModelPattern],
    kws: &HashSet<String>,
) -> Result<TokenStream> {
    let mut steps = Vec::new();
    for p in patterns {
        steps.push(generate_pattern_step(p, kws)?);
    }
    Ok(quote! { #(#steps)* })
}

fn generate_pattern_step(pattern: &ModelPattern, kws: &HashSet<String>) -> Result<TokenStream> {
    match pattern {
        ModelPattern::Cut(_) => Ok(quote!()),
        ModelPattern::Lit { binding, lit } => {
            if let Lit::Str(lit) = lit {
                let token_types = analysis::resolve_token_types(lit, kws)?;

                if token_types.len() <= 1 {
                    let parses = token_types.iter().map(|ty| {
                        if let Some(bind) = binding {
                            quote! {
                                let #bind = input.parse::<#ty>()?;
                                ctx.record_span(syn::spanned::Spanned::span(&#bind));
                            }
                        } else {
                            quote! {
                                let _t = input.parse::<#ty>()?;
                                ctx.record_span(syn::spanned::Spanned::span(&_t));
                            }
                        }
                    });
                    Ok(quote! { #(#parses)* })
                } else {
                    let mut steps = Vec::new();
                    let mut checks = Vec::new();
                    let mut results = Vec::new();

                    for (i, ty) in token_types.iter().enumerate() {
                        let var = format_ident!("_t{}", i);
                        steps.push(quote! {
                            let #var = input.parse::<#ty>()?;
                        });
                        results.push(var.clone());

                        // Record span for the last token
                        if i == token_types.len() - 1 {
                            steps.push(quote! {
                                ctx.record_span(syn::spanned::Spanned::span(&#var));
                            });
                        }

                        if i > 0 {
                            let prev = format_ident!("_t{}", i - 1);
                            let err_msg =
                                format!("expected '{}', found space between tokens", lit.value());
                            checks.push(quote! {
                                if #prev.span().end() != #var.span().start() {
                                    return Err(syn::Error::new(
                                        #var.span(),
                                        #err_msg
                                    ));
                                }
                            });
                        }
                    }

                    let bind_stmt = if let Some(bind) = binding {
                        quote! { let #bind = ( #(#results),* ); }
                    } else {
                        quote! {}
                    };

                    Ok(quote! {
                        {
                            #(#steps)*
                            #(#checks)*
                            #bind_stmt
                        }
                    })
                }
            } else {
                Err(syn::Error::new(
                    lit.span(),
                    "Non-string literals are not supported as matchers.",
                ))
            }
        }
        ModelPattern::RuleCall {
            binding,
            rule_name,
            generics,
            args,
        } => {
            let rule_name_str = rule_name.to_string();
            let builtins = SynBackend::get_builtins();
            let is_builtin = builtins.iter().any(|b| b.name == rule_name_str);

            if rule_name_str == "separated" {
                // separated(rule, sep, min=0, trailing=false)
                if args.len() < 2 {
                    return Err(syn::Error::new(
                        rule_name.span(),
                        "separated requires at least 2 arguments: (rule, separator)",
                    ));
                }

                let rule_arg = match &args[0] {
                    Argument::Positional(p) => p,
                    Argument::Named(_, p) => p,
                };
                let sep_arg = match &args[1] {
                    Argument::Positional(p) => p,
                    Argument::Named(_, p) => p,
                };

                let mut min = 0usize;
                let mut trailing = false;

                // Parse optional args
                for arg in &args[2..] {
                    match arg {
                        Argument::Named(id, val) => {
                            if id == "min" {
                                if let ModelPattern::Lit {
                                    lit: Lit::Int(i), ..
                                } = val
                                {
                                    min = i.base10_parse()?;
                                }
                            } else if id == "trailing" {
                                if let ModelPattern::Lit {
                                    lit: Lit::Bool(b), ..
                                } = val
                                {
                                    trailing = b.value;
                                }
                            }
                        }
                        Argument::Positional(val) => {
                            // Assume positional min
                            if let ModelPattern::Lit {
                                lit: Lit::Int(i), ..
                            } = val
                            {
                                min = i.base10_parse()?;
                            }
                        }
                    }
                }

                let container_ty = if let Some(ty) = generics.first() {
                    quote!(#ty)
                } else {
                    quote!(Vec)
                };

                // Inject binding if missing
                let (rule_arg_with_binding, item_binding) = match rule_arg {
                    ModelPattern::RuleCall {
                        binding: None,
                        rule_name,
                        generics,
                        args,
                    } => {
                        let temp = format_ident!("_item");
                        let new_pat = ModelPattern::RuleCall {
                            binding: Some(temp.clone()),
                            rule_name: rule_name.clone(),
                            generics: generics.clone(),
                            args: args.clone(),
                        };
                        (new_pat, vec![temp])
                    }
                    ModelPattern::Lit { binding: None, lit } => {
                        let temp = format_ident!("_item");
                        let new_pat = ModelPattern::Lit {
                            binding: Some(temp.clone()),
                            lit: lit.clone(),
                        };
                        (new_pat, vec![temp])
                    }
                    _ => (
                        rule_arg.clone(),
                        analysis::collect_bindings(std::slice::from_ref(rule_arg)),
                    ),
                };

                let rule_parser = generate_pattern_step(&rule_arg_with_binding, kws)?;
                let sep_parser = generate_pattern_step(sep_arg, kws)?;
                let sep_peek = analysis::get_simple_peek(sep_arg, kws).ok().flatten();

                let push_stmt = if item_binding.len() == 1 {
                    let b = &item_binding[0];
                    quote! { _items.push(#b); }
                } else if item_binding.is_empty() {
                    quote! { _items.push(()); }
                } else {
                    let b = &item_binding;
                    quote! { _items.push((#(#b),*)); }
                };

                let sep_logic = if let Some(peek) = sep_peek {
                    quote! {
                        if input.peek(#peek) {
                            #sep_parser
                            true
                        } else {
                            false
                        }
                    }
                } else {
                    quote! {
                        if rt::attempt(input, ctx, |mut input, ctx| { #sep_parser Ok(()) })?.is_some() {
                            true
                        } else {
                            false
                        }
                    }
                };

                let refined_loop = quote! {
                    let mut _items = #container_ty::new();
                    let mut _first = true;
                    loop {
                        if !_first {
                            // Expect separator
                            if !{#sep_logic} {
                                break;
                            }
                        }

                        // Attempt parse item
                        let _checkpoint = input.cursor();
                        let _item_res = rt::attempt(input, ctx, |mut input, ctx| {
                             #rule_parser
                             Ok( (#(#item_binding),*) )
                        })?;

                        if let Some(val) = _item_res {
                            let (#(#item_binding),*) = val;
                            #push_stmt
                            _first = false;
                        } else {
                            if !_first && !#trailing {
                                // Clear best error because we want to report specific error
                                let _ = ctx.take_best_error();
                                return Err(input.error("expected item after separator"));
                            }
                            break;
                        }
                    }
                    if _items.len() < (#min as usize) {
                        // Clear best error because we want to report logic error
                        let _ = ctx.take_best_error();
                        return Err(input.error(concat!("expected at least ", #min, " items")));
                    }
                    _items
                };

                if let Some(bind) = binding {
                    Ok(quote! { let #bind = { #refined_loop }; })
                } else {
                    Ok(quote! { let _ = { #refined_loop }; })
                }
            } else if rule_name_str == "repeated" {
                // repeated(rule, min=0)
                if args.is_empty() {
                    return Err(syn::Error::new(
                        rule_name.span(),
                        "repeated requires at least 1 argument: (rule)",
                    ));
                }
                let rule_arg = match &args[0] {
                    Argument::Positional(p) => p,
                    Argument::Named(_, p) => p,
                };

                let mut min = 0usize;
                // Parse optional args
                for arg in &args[1..] {
                    match arg {
                        Argument::Named(id, val) => {
                            if id == "min" {
                                if let ModelPattern::Lit {
                                    lit: Lit::Int(i), ..
                                } = val
                                {
                                    min = i.base10_parse()?;
                                }
                            }
                        }
                        Argument::Positional(val) => {
                            if let ModelPattern::Lit {
                                lit: Lit::Int(i), ..
                            } = val
                            {
                                min = i.base10_parse()?;
                            }
                        }
                    }
                }

                let container_ty = if let Some(ty) = generics.first() {
                    quote!(#ty)
                } else {
                    quote!(Vec)
                };

                // Inject binding if missing
                let (rule_arg_with_binding, item_binding) = match rule_arg {
                    ModelPattern::RuleCall {
                        binding: None,
                        rule_name,
                        generics,
                        args,
                    } => {
                        let temp = format_ident!("_item");
                        let new_pat = ModelPattern::RuleCall {
                            binding: Some(temp.clone()),
                            rule_name: rule_name.clone(),
                            generics: generics.clone(),
                            args: args.clone(),
                        };
                        (new_pat, vec![temp])
                    }
                    ModelPattern::Lit { binding: None, lit } => {
                        let temp = format_ident!("_item");
                        let new_pat = ModelPattern::Lit {
                            binding: Some(temp.clone()),
                            lit: lit.clone(),
                        };
                        (new_pat, vec![temp])
                    }
                    _ => (
                        rule_arg.clone(),
                        analysis::collect_bindings(std::slice::from_ref(rule_arg)),
                    ),
                };

                let rule_parser = generate_pattern_step(&rule_arg_with_binding, kws)?;

                let push_stmt = if item_binding.len() == 1 {
                    let b = &item_binding[0];
                    quote! { _items.push(#b); }
                } else if item_binding.is_empty() {
                    quote! { _items.push(()); }
                } else {
                    let b = &item_binding;
                    quote! { _items.push((#(#b),*)); }
                };

                let loop_logic = quote! {
                    let mut _items = #container_ty::new();
                    while let Some(val) = rt::attempt(input, ctx, |mut input, ctx| {
                        #rule_parser
                        Ok( (#(#item_binding),*) )
                    })? {
                        let (#(#item_binding),*) = val;
                        #push_stmt
                    }
                     if _items.len() < (#min as usize) {
                        // Clear best error
                        let _ = ctx.take_best_error();
                        return Err(input.error(concat!("expected at least ", #min, " items")));
                    }
                    _items
                };

                if let Some(bind) = binding {
                    Ok(quote! { let #bind = { #loop_logic }; })
                } else {
                    Ok(quote! { let _ = { #loop_logic }; })
                }
            } else if is_builtin {
                // Generate a token-filtering expression for the primitive.
                let expr = match rule_name_str.as_str() {
                    "alpha" => quote! {
                        {
                            let t = rt::token_filter::alpha(input)?;
                            ctx.record_span(syn::spanned::Spanned::span(&t));
                            t
                        }
                    },
                    "digit" => quote! {
                        {
                            let t = rt::token_filter::digit(input)?;
                            ctx.record_span(syn::spanned::Spanned::span(&t));
                            t
                        }
                    },
                    "alphanumeric" => quote! {
                        {
                            let t = rt::token_filter::alphanumeric(input)?;
                            ctx.record_span(syn::spanned::Spanned::span(&t));
                            t
                        }
                    },
                    "hex_digit" => quote! {
                        {
                            let t = rt::token_filter::hex_digit(input)?;
                            ctx.record_span(syn::spanned::Spanned::span(&t));
                            t
                        }
                    },
                    "oct_digit" => quote! {
                        {
                            let t = rt::token_filter::oct_digit(input)?;
                            ctx.record_span(syn::spanned::Spanned::span(&t));
                            t
                        }
                    },
                    "any_byte" => quote! {
                        {
                            let t = input.parse::<syn::LitByte>()?;
                            ctx.record_span(syn::spanned::Spanned::span(&t));
                            t
                        }
                    },
                    "eof" => {
                        return Ok(quote! {
                            if !input.is_empty() {
                                return Err(syn::Error::new(input.span(), "expected end of input"));
                            }
                        });
                    }
                    "fail" => {
                        let arg_expr = if let Some(arg) = args.first() {
                            match arg {
                                Argument::Positional(ModelPattern::Lit {
                                    lit: syn::Lit::Str(s),
                                    ..
                                }) => s.value(),
                                _ => "Explicit failure".to_string(),
                            }
                        } else {
                            "Explicit failure".to_string()
                        };

                        return Ok(quote! {
                            if true {
                                return Err(syn::Error::new(input.span(), #arg_expr));
                            }
                        });
                    }
                    "whitespace" => {
                        return Ok(quote! {
                            if !ctx.check_whitespace(input.span()) {
                                return Err(syn::Error::new(input.span(), "expected whitespace"));
                            }
                        });
                    }
                    // Defer to built-in rules for high-level primitives like "ident", "integer", "float"
                    _ => {
                        let func_call = generate_rule_call_expr(rule_name, args);
                        quote! { #func_call }
                    }
                };

                let result = if let Some(bind) = binding {
                    quote! { let #bind = #expr; }
                } else {
                    quote! { let _ = #expr; }
                };
                Ok(result)
            } else {
                let func_call = generate_rule_call_expr(rule_name, args);
                Ok(if let Some(bind) = binding {
                    quote! { let #bind = #func_call; }
                } else {
                    quote! { let _ = #func_call; }
                })
            }
        }

        ModelPattern::Repeat(inner, _) => {
            let bindings = analysis::collect_bindings(std::slice::from_ref(inner));

            if !bindings.is_empty() {
                // Use temporary names for vectors to avoid shadowing by inner bindings
                let vec_names: Vec<_> = bindings
                    .iter()
                    .map(|b| format_ident!("_vec_{}", b))
                    .collect();

                let init_vecs: Vec<_> = vec_names
                    .iter()
                    .map(|v| quote!(let mut #v = Vec::new();))
                    .collect();
                let push_vecs: Vec<_> = vec_names
                    .iter()
                    .zip(bindings.iter())
                    .map(|(v, b)| quote!(#v.push(#b);))
                    .collect();
                let finalize_vecs: Vec<_> = bindings
                    .iter()
                    .zip(vec_names.iter())
                    .map(|(b, v)| quote!(let #b = #v;))
                    .collect();

                let inner_logic = generate_pattern_step(inner, kws)?;

                // Only use peek optimization if it's safe and unambiguous
                let peek_opt = analysis::get_simple_peek(inner, kws).ok().flatten();

                if let Some(peek) = peek_opt {
                    Ok(quote! {
                       #(#init_vecs)*
                       while input.peek(#peek) {
                           {
                               #inner_logic
                               #(#push_vecs)*
                           }
                       }
                       #(#finalize_vecs)*
                    })
                } else {
                    let return_tuple = quote!(( #(#bindings),* ));
                    let tuple_pat = quote!(( #(#bindings),* ));

                    Ok(quote! {
                       #(#init_vecs)*
                       // Pass ctx to attempt
                       while let Some(vals) = rt::attempt(input, ctx, |mut input, ctx| {
                           #inner_logic
                           Ok(#return_tuple)
                       })? {
                           let #tuple_pat = vals;
                           #(#push_vecs)*
                       }
                       #(#finalize_vecs)*
                    })
                }
            } else {
                let inner_logic = generate_pattern_step(inner, kws)?;
                Ok(quote! {
                    // Pass ctx to attempt
                    while let Some(_) = rt::attempt(input, ctx, |mut input, ctx| { #inner_logic Ok(()) })? {}
                })
            }
        }

        ModelPattern::Plus(inner, _) => {
            let bindings = analysis::collect_bindings(std::slice::from_ref(inner));

            if !bindings.is_empty() {
                // Use temporary names for vectors to avoid shadowing by inner bindings
                let vec_names: Vec<_> = bindings
                    .iter()
                    .map(|b| format_ident!("_vec_{}", b))
                    .collect();

                let init_vecs: Vec<_> = vec_names
                    .iter()
                    .map(|v| quote!(let mut #v = Vec::new();))
                    .collect();
                let push_vecs: Vec<_> = vec_names
                    .iter()
                    .zip(bindings.iter())
                    .map(|(v, b)| quote!(#v.push(#b);))
                    .collect();
                let finalize_vecs: Vec<_> = bindings
                    .iter()
                    .zip(vec_names.iter())
                    .map(|(b, v)| quote!(let #b = #v;))
                    .collect();

                let inner_logic = generate_pattern_step(inner, kws)?;
                let peek_opt = analysis::get_simple_peek(inner, kws).ok().flatten();

                if let Some(peek) = peek_opt {
                    Ok(quote! {
                       #(#init_vecs)*
                       {
                           #inner_logic
                           #(#push_vecs)*
                       }
                       while input.peek(#peek) {
                           {
                               #inner_logic
                               #(#push_vecs)*
                           }
                       }
                       #(#finalize_vecs)*
                    })
                } else {
                    let return_tuple = quote!(( #(#bindings),* ));
                    let tuple_pat = quote!(( #(#bindings),* ));

                    Ok(quote! {
                       #(#init_vecs)*
                       {
                           #inner_logic
                           #(#push_vecs)*
                       }
                       // Pass ctx to attempt
                       while let Some(vals) = rt::attempt(input, ctx, |mut input, ctx| {
                           #inner_logic
                           Ok(#return_tuple)
                       })? {
                           let #tuple_pat = vals;
                           #(#push_vecs)*
                       }
                       #(#finalize_vecs)*
                    })
                }
            } else {
                let inner_logic = generate_pattern_step(inner, kws)?;
                Ok(quote! {
                    #inner_logic
                    // Pass ctx to attempt
                    while let Some(_) = rt::attempt(input, ctx, |mut input, ctx| { #inner_logic Ok(()) })? {}
                })
            }
        }

        ModelPattern::Optional(inner, _) => {
            let inner_logic = generate_pattern_step(inner, kws)?;
            let peek_opt = analysis::get_simple_peek(inner, kws).ok().flatten();
            let is_nullable = analysis::is_nullable(inner);

            let bindings = analysis::collect_bindings(std::slice::from_ref(inner));

            if let (Some(peek), false) = (peek_opt, is_nullable) {
                if bindings.is_empty() {
                    Ok(quote! {
                        if input.peek(#peek) {
                            // Pass ctx to attempt
                            let _ = rt::attempt(input, ctx, |mut input, ctx| { #inner_logic Ok(()) })?;
                        }
                    })
                } else {
                    // For optional binding, we need to return Option<T>
                    let vars: Vec<_> = bindings.iter().map(|b| quote!(#b)).collect();
                    let some_vars: Vec<_> = bindings.iter().map(|b| quote!(Some(#b))).collect();
                    let none_vars: Vec<_> = bindings.iter().map(|_| quote!(None)).collect();

                    Ok(quote! {
                        let (#(#vars),*) = if input.peek(#peek) {
                            if let Some(vals) = rt::attempt(input, ctx, |mut input, ctx| {
                                #inner_logic
                                Ok((#(#vars),*))
                            })? {
                                let (#(#vars),*) = vals;
                                (#(#some_vars),*)
                            } else {
                                (#(#none_vars),*)
                            }
                        } else {
                            (#(#none_vars),*)
                        };
                    })
                }
            } else if bindings.is_empty() {
                Ok(quote! {
                    // Pass ctx to attempt
                    let _ = rt::attempt(input, ctx, |mut input, ctx| { #inner_logic Ok(()) })?;
                })
            } else {
                let vars: Vec<_> = bindings.iter().map(|b| quote!(#b)).collect();
                let some_vars: Vec<_> = bindings.iter().map(|b| quote!(Some(#b))).collect();
                let none_vars: Vec<_> = bindings.iter().map(|_| quote!(None)).collect();

                Ok(quote! {
                    let (#(#vars),*) = if let Some(vals) = rt::attempt(input, ctx, |mut input, ctx| {
                            #inner_logic
                            Ok((#(#vars),*))
                    })? {
                        let (#(#vars),*) = vals;
                        (#(#some_vars),*)
                    } else {
                        (#(#none_vars),*)
                    };
                })
            }
        }
        ModelPattern::Group(alts, _) => {
            use super::rule::generate_variants_internal;

            let temp_variants = alts
                .iter()
                .map(|pat_seq| {
                    let bindings = analysis::collect_bindings(pat_seq);
                    let action_expr = if bindings.is_empty() {
                        quote!(())
                    } else {
                        quote!(( #(#bindings),* ))
                    };
                    RuleVariant {
                        pattern: pat_seq.clone(),
                        action: quote!({ #action_expr }),
                    }
                })
                .collect::<Vec<_>>();

            let variant_logic = generate_variants_internal(&temp_variants, false, kws)?;
            let group_bindings = analysis::collect_bindings(std::slice::from_ref(pattern));

            let wrapped_logic = quote! {
                (|| -> syn::Result<_> {
                    #variant_logic
                })()
            };

            if group_bindings.is_empty() {
                Ok(quote! { { #wrapped_logic }?; })
            } else {
                let tuple_pat = quote!(( #(#group_bindings),* ));
                Ok(quote! {
                    let #tuple_pat = { #wrapped_logic }?;
                })
            }
        }

        ModelPattern::Bracketed(s, _)
        | ModelPattern::Braced(s, _)
        | ModelPattern::Parenthesized(s, _) => {
            let macro_name = match pattern {
                ModelPattern::Bracketed(_, _) => quote!(bracketed),
                ModelPattern::Braced(_, _) => quote!(braced),
                _ => quote!(parenthesized),
            };

            let inner_logic = generate_sequence_steps(s, kws)?;
            let bindings = analysis::collect_bindings(s);

            if bindings.is_empty() {
                Ok(quote! { {
                    let content;
                    let _ = syn::#macro_name!(content in input);
                    // TODO: Record span of brackets?
                    let input = &content; // This shadows outer input.
                    // But `syn::bracketed!` (etc) assigns `ParseBuffer` to `content`.
                    // And `let input = &content`.
                    // `content` is `ParseBuffer`.
                    // `input` is `&ParseBuffer` (ParseStream).
                    // This `input` is immutable.
                    // If we call `parse_*_impl(&mut input, ...)`, we need `mut input`.
                    // So we must shadow with `let mut input = &content;`
                    let mut input = &content;
                    #inner_logic
                }})
            } else if bindings.len() == 1 {
                let bind = &bindings[0];
                Ok(quote! {
                    let #bind = {
                        let content;
                        let _ = syn::#macro_name!(content in input);
                        let mut input = &content;
                        #inner_logic
                        #bind
                    };
                })
            } else {
                Ok(quote! {
                    let (#(#bindings),*) = {
                        let content;
                        let _ = syn::#macro_name!(content in input);
                        let mut input = &content;
                        #inner_logic
                        (#(#bindings),*)
                    };
                })
            }
        }

        ModelPattern::SpanBinding(inner, span_var, _) => {
            let (inner_pat, binding_name) = match &**inner {
                ModelPattern::RuleCall {
                    binding,
                    rule_name,
                    generics,
                    args,
                } => {
                    if let Some(b) = binding {
                        (inner.clone(), b.clone())
                    } else {
                        let temp = format_ident!("_val_{}", span_var);
                        let new_inner = ModelPattern::RuleCall {
                            binding: Some(temp.clone()),
                            rule_name: rule_name.clone(),
                            generics: generics.clone(),
                            args: args.clone(),
                        };
                        (Box::new(new_inner), temp)
                    }
                }
                ModelPattern::Recover {
                    binding,
                    body,
                    sync,
                    span,
                } => {
                    if let Some(b) = binding {
                        (inner.clone(), b.clone())
                    } else {
                        let temp = format_ident!("_val_{}", span_var);
                        let new_inner = ModelPattern::Recover {
                            binding: Some(temp.clone()),
                            body: body.clone(),
                            sync: sync.clone(),
                            span: *span,
                        };
                        (Box::new(new_inner), temp)
                    }
                }
                ModelPattern::Lit { binding, lit } => {
                    if let Some(b) = binding {
                        (inner.clone(), b.clone())
                    } else {
                        let temp = format_ident!("_val_{}", span_var);
                        let new_inner = ModelPattern::Lit {
                            binding: Some(temp.clone()),
                            lit: lit.clone(),
                        };
                        (Box::new(new_inner), temp)
                    }
                }
                _ => {
                    return Err(syn::Error::new(
                        span_var.span(),
                        "Span binding (@) is currently only supported on rule calls, recover() blocks and literals.",
                    ));
                }
            };

            let inner_code = generate_pattern_step(&inner_pat, kws)?;

            Ok(quote! {
                #inner_code
                let #span_var = syn::spanned::Spanned::span(&#binding_name);
            })
        }

        ModelPattern::Recover {
            binding,
            body,
            sync,
            span,
        } => {
            let effective_body = if let Some(bind) = binding {
                match &**body {
                    ModelPattern::RuleCall {
                        binding: None,
                        rule_name,
                        generics,
                        args,
                    } => Box::new(ModelPattern::RuleCall {
                        binding: Some(bind.clone()),
                        rule_name: rule_name.clone(),
                        generics: generics.clone(),
                        args: args.clone(),
                    }),
                    // If the body is already binding, we might have an issue if we try to override it.
                    // But typically recover wraps a rule call.
                    _ => body.clone(), // fallback
                }
            } else {
                body.clone()
            };

            let inner_logic = generate_pattern_step(&effective_body, kws)?;
            let sync_peek = analysis::get_simple_peek(sync, kws)?.ok_or_else(|| {
                syn::Error::new(
                    sync.span(),
                    "Sync pattern in recover(...) must have a simple start token.",
                )
            })?;

            let bindings = analysis::collect_bindings(std::slice::from_ref(&effective_body));

            // Fix: Mark span as unused to silence clippy warning
            let _ = span;

            if bindings.is_empty() {
                Ok(quote! {
                    // Pass ctx to attempt_recover
                    if rt::attempt_recover(input, ctx, |mut input, ctx| { #inner_logic Ok(()) })?.is_none() {
                        rt::skip_until(input, |i| i.peek(#sync_peek))?;
                    }
                })
            } else {
                let none_exprs = bindings.iter().map(|_| quote!(Option::<_>::None));
                // We need to return Option<T> for each binding if it failed.

                if let Some(main_bind) = binding {
                    Ok(quote! {
                        let #main_bind = match rt::attempt_recover(input, ctx, |mut input, ctx| {
                            #inner_logic
                            Ok((#(#bindings),*))
                        })? {
                            Some(vals) => {
                                let (#(#bindings),*) = vals;
                                Some(#(#bindings),*)
                            },
                            None => {
                                rt::skip_until(input, |i| i.peek(#sync_peek))?;
                                None
                            }
                        };
                    })
                } else {
                    // Fallback to tuple destructuring if no single binding on recover
                    Ok(quote! {
                        let (#(#bindings),*) = match rt::attempt_recover(input, ctx, |mut input, ctx| {
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
            }
        }

        ModelPattern::Peek(inner, _) => {
            let bindings = analysis::collect_bindings(std::slice::from_ref(inner));
            let inner_logic = generate_pattern_step(inner, kws)?;

            if bindings.is_empty() {
                Ok(quote! {
                   let _ = rt::peek(input, ctx, |mut input, ctx| {
                       #inner_logic
                       Ok(())
                   })?;
                })
            } else {
                let tuple_pat = quote!(( #(#bindings),* ));
                let tuple_ret = quote!(( #(#bindings),* ));

                Ok(quote! {
                    let #tuple_pat = rt::peek(input, ctx, |mut input, ctx| {
                        #inner_logic
                        Ok(#tuple_ret)
                    })?;
                })
            }
        }

        ModelPattern::Not(inner, _) => {
            // Not does not export bindings.
            let inner_logic = generate_pattern_step(inner, kws)?;
            Ok(quote! {
                rt::not_check(input, ctx, |mut input, ctx| {
                    #inner_logic
                    Ok(())
                })?;
            })
        }

        ModelPattern::Until {
            binding, pattern, ..
        } => {
            let inner_logic = generate_pattern_step(pattern, kws)?;

            let loop_body = quote! {
                let mut _tokens = Vec::new();
                while !input.is_empty() {
                    let is_match = rt::peek(input, ctx, |mut input, ctx| {
                         #inner_logic
                         Ok(())
                    }).is_ok();

                    if is_match {
                        break;
                    }

                    let t: proc_macro2::TokenTree = input.parse()?;
                    _tokens.push(t);
                }
                proc_macro2::TokenStream::from_iter(_tokens)
            };

            if let Some(bind) = binding {
                Ok(quote! { let #bind = { #loop_body }; })
            } else {
                Ok(quote! { let _ = { #loop_body }; })
            }
        }
    }
}

fn generate_rule_call_expr(rule_name: &syn::Ident, args: &[Argument]) -> TokenStream {
    // Call the _impl version and pass ctx
    let f = format_ident!("parse_{}_impl", rule_name);

    let arg_exprs: Vec<TokenStream> = args
        .iter()
        .map(|arg| match arg {
            Argument::Positional(p) | Argument::Named(_, p) => match p {
                ModelPattern::Lit { lit, .. } => quote!(#lit),
                ModelPattern::RuleCall {
                    rule_name, args, ..
                } if args.is_empty() => quote!(#rule_name),
                _ => quote!(compile_error!("Complex pattern used as runtime argument")),
            },
        })
        .collect();

    if arg_exprs.is_empty() {
        quote!(#f(&mut input, ctx)?)
    } else {
        quote!(#f(&mut input, ctx, #(#arg_exprs),*)?)
    }
}
