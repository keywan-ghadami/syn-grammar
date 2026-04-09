use crate::backend::SynBackend;
use crate::codegen::CodegenContext;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::spanned::Spanned;
use syn::{Lit, Result};
use syn_grammar_model::{analysis, model::*, Backend};

pub fn generate_sequence(
    patterns: &[ModelPattern],
    action: &TokenStream,
    ctx: &CodegenContext,
) -> Result<TokenStream> {
    let steps = generate_sequence_steps(patterns, ctx)?;
    Ok(quote! {
        {
            #steps
            // CRITICAL FIX: Semantische Action-Blöcke kapseln.
            // Fängt der Nutzer-Code mit Err(...) ab, stufen wir das zwingend als Strukturfehler ein.
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
        }
    })
}

pub fn generate_sequence_steps(
    patterns: &[ModelPattern],
    ctx: &CodegenContext,
) -> Result<TokenStream> {
    let mut steps = Vec::new();
    for p in patterns {
        steps.push(generate_pattern_step(p, ctx)?);
    }
    Ok(quote! { #(#steps)* })
}

fn generate_pattern_step(pattern: &ModelPattern, ctx: &CodegenContext) -> Result<TokenStream> {
    match pattern {
        ModelPattern::Cut(_) => Ok(quote!()),
        ModelPattern::Lit { binding, lit } => {
            if let Lit::Str(lit) = lit {
                let token_types = analysis::resolve_token_types(lit, ctx.custom_keywords)?;

                if token_types.len() <= 1 {
                    let parses = token_types.iter().map(|ty| {
                        if let Some(bind) = binding {
                            quote! {
                                let #bind = input.parse::<#ty>()?;
                                ctx.record_span(syn::spanned::Spanned::span(&#bind))?;
                            }
                        } else {
                            quote! {
                                let _t = input.parse::<#ty>()?;
                                ctx.record_span(syn::spanned::Spanned::span(&_t))?;
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
                                ctx.record_span(syn::spanned::Spanned::span(&#var))?;
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
            rule_path,
            generics,
            args,
        } => {
            let rule_name_ident = rule_path.get_ident();
            let builtins = SynBackend::get_builtins();
            let is_builtin =
                rule_name_ident.is_some_and(|ident| builtins.iter().any(|b| ident == b.name));

            let first_segment = rule_path.segments.iter().next();
            let is_syn_type = if let Some(seg) = first_segment {
                seg.ident == "syn"
            } else {
                false
            };

            if is_syn_type {
                let expr = quote! {
                    {
                        let start_span = input.span();
                        eprintln!("[TRACE] Attempting to parse syn type: {} at {:?}", stringify!(#rule_path), start_span);
                        match input.parse::<#rule_path>() {
                            Ok(val) => {
                                eprintln!("[TRACE] Successfully parsed syn type: {}", stringify!(#rule_path));
                                Ok(val)
                            },
                            Err(e) => {
                                eprintln!("[TRACE] FAILED to parse syn type: {}. Error: '{}', Error Span: {:?}", stringify!(#rule_path), e.to_string(), e.span());
                                eprintln!("[TRACE] Start span was: {:?}", start_span);
                                if e.span().start() > start_span.start() {
                                    eprintln!("[TRACE] Progress was made. Calling trigger_fail()");
                                    ctx.trigger_fail();
                                } else {
                                    eprintln!("[TRACE] No progress. NOT calling trigger_fail()");
                                }
                                Err(e)
                            }
                        }
                    }?
                };
                if let Some(bind) = binding {
                    Ok(quote! { let #bind = #expr; })
                } else {
                    Ok(quote! { let _ = #expr; })
                }
            } else if rule_path.is_ident("separated") {
                // separated(rule, sep, min=0, trailing=false)
                if args.len() < 2 {
                    return Err(syn::Error::new(
                        rule_path.span(),
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
                let mut custom_error: Option<String> = None;

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
                            } else if id == "error" || id == "err" {
                                if let ModelPattern::Lit {
                                    lit: Lit::Str(s), ..
                                } = val
                                {
                                    custom_error = Some(s.value());
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
                    match ty {
                        syn::Type::Infer(_) => quote!(Vec),
                        _ => quote!(#ty),
                    }
                } else {
                    quote!(Vec)
                };

                // Inject binding if missing
                let (rule_arg_with_binding, item_binding) = match rule_arg {
                    ModelPattern::RuleCall {
                        binding: None,
                        rule_path,
                        generics,
                        args,
                    } => {
                        let temp = format_ident!("_item");
                        let new_pat = ModelPattern::RuleCall {
                            binding: Some(temp.clone()),
                            rule_path: rule_path.clone(),
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

                let rule_parser = generate_pattern_step(&rule_arg_with_binding, ctx)?;
                let sep_parser = generate_pattern_step(sep_arg, ctx)?;

                let item_return_expr = if item_binding.len() == 1 {
                    let b = &item_binding[0];
                    quote! { #b }
                } else if item_binding.is_empty() {
                    quote! { () }
                } else {
                    let b = &item_binding;
                    quote! { (#(#b),*) }
                };

                let error_msg_expr = if let Some(msg) = custom_error {
                    quote!(Some(#msg))
                } else {
                    quote!(None)
                };

                let refined_logic = quote! {
                    let _items_vec = rt::parse_separated::<_, _, _>(
                        input,
                        ctx,
                        |mut input, ctx| {
                             #rule_parser
                             Ok(#item_return_expr)
                        },
                        |mut input, ctx| {
                             #sep_parser
                             Ok(())
                        },
                        #min,
                        #trailing,
                        #error_msg_expr
                    )?;
                    // Convert to container type if needed (currently always Vec, but could be adapted)
                    let mut _items = #container_ty::from_iter(_items_vec);
                    _items
                };

                if let Some(bind) = binding {
                    Ok(quote! { let #bind = { #refined_logic }; })
                } else {
                    Ok(quote! { let _ = { #refined_logic }; })
                }
            } else if rule_path.is_ident("repeated") {
                // repeated(rule, min=0)
                if args.is_empty() {
                    return Err(syn::Error::new(
                        rule_path.span(),
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
                    match ty {
                        syn::Type::Infer(_) => quote!(Vec),
                        _ => quote!(#ty),
                    }
                } else {
                    quote!(Vec)
                };

                // Inject binding if missing
                let (rule_arg_with_binding, item_binding) = match rule_arg {
                    ModelPattern::RuleCall {
                        binding: None,
                        rule_path,
                        generics,
                        args,
                    } => {
                        let temp = format_ident!("_item");
                        let new_pat = ModelPattern::RuleCall {
                            binding: Some(temp.clone()),
                            rule_path: rule_path.clone(),
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

                let rule_parser = generate_pattern_step(&rule_arg_with_binding, ctx)?;

                let item_return_expr = if item_binding.len() == 1 {
                    let b = &item_binding[0];
                    quote! { #b }
                } else if item_binding.is_empty() {
                    quote! { () }
                } else {
                    let b = &item_binding;
                    quote! { (#(#b),*) }
                };

                let loop_logic = quote! {
                    let _items_vec = rt::parse_repeated::<_, _>(
                        input,
                        ctx,
                        |mut input, ctx| {
                             #rule_parser
                             Ok(#item_return_expr)
                        },
                        #min
                    )?;
                    let mut _items = #container_ty::from_iter(_items_vec);
                    _items
                };

                if let Some(bind) = binding {
                    Ok(quote! { let #bind = { #loop_logic }; })
                } else {
                    Ok(quote! { let _ = { #loop_logic }; })
                }
            } else if is_builtin {
                let rule_name_str = rule_name_ident.unwrap().to_string();
                // Generate a token-filtering expression for the primitive.
                let expr = match rule_name_str.as_str() {
                    "alpha" => quote! {
                        {
                            let t = rt::token_filter::alpha(input)?;
                            ctx.record_span(syn::spanned::Spanned::span(&t))?;
                            t
                        }
                    },
                    "digit" => quote! {
                        {
                            let t = rt::token_filter::digit(input)?;
                            ctx.record_span(syn::spanned::Spanned::span(&t))?;
                            t
                        }
                    },
                    "alphanumeric" => quote! {
                        {
                            let t = rt::token_filter::alphanumeric(input)?;
                            ctx.record_span(syn::spanned::Spanned::span(&t))?;
                            t
                        }
                    },
                    "hex_digit" => quote! {
                        {
                            let t = rt::token_filter::hex_digit(input)?;
                            ctx.record_span(syn::spanned::Spanned::span(&t))?;
                            t
                        }
                    },
                    "oct_digit" => quote! {
                        {
                            let t = rt::token_filter::oct_digit(input)?;
                            ctx.record_span(syn::spanned::Spanned::span(&t))?;
                            t
                        }
                    },
                    "any_byte" => quote! {
                        {
                            let t = input.parse::<syn::LitByte>()?;
                            ctx.record_span(syn::spanned::Spanned::span(&t))?;
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
                    // "fail" removed here - handled by ModelPattern::Fail
                    "whitespace" => {
                        return Ok(quote! {
                            if !ctx.check_whitespace(input.span()) {
                                return Err(syn::Error::new(input.span(), "expected whitespace"));
                            }
                        });
                    }
                    // Defer to built-in rules for high-level primitives like "ident", "integer", "float"
                    _ => {
                        let impl_name = format_ident!("parse_{}_impl", rule_name_ident.unwrap());
                        quote! { syn_grammar::builtins::#impl_name(&mut input, ctx)? }
                    }
                };

                let result = if let Some(bind) = binding {
                    quote! { let #bind = #expr; }
                } else {
                    quote! { let _ = #expr; }
                };
                Ok(result)
            } else {
                let func_call = generate_rule_call_expr(rule_path, args, ctx)?;
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

                let inner_logic = generate_pattern_step(inner, ctx)?;

                // Only use peek optimization if it's safe and unambiguous
                let peek_opt = analysis::get_simple_peek(inner, ctx.custom_keywords)
                    .ok()
                    .flatten();

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
                let inner_logic = generate_pattern_step(inner, ctx)?;
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

                let inner_logic = generate_pattern_step(inner, ctx)?;
                let peek_opt = analysis::get_simple_peek(inner, ctx.custom_keywords)
                    .ok()
                    .flatten();

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
                let inner_logic = generate_pattern_step(inner, ctx)?;
                Ok(quote! {
                    #inner_logic
                    // Pass ctx to attempt
                    while let Some(_) = rt::attempt(input, ctx, |mut input, ctx| { #inner_logic Ok(()) })? {}
                })
            }
        }

        ModelPattern::Optional(inner, _) => {
            let inner_logic = generate_pattern_step(inner, ctx)?;
            let peek_opt = analysis::get_simple_peek(inner, ctx.custom_keywords)
                .ok()
                .flatten();
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
        ModelPattern::Group { binding, alts, .. } => {
            use super::rule::generate_variants_internal;

            let temp_variants = alts
                .iter()
                .map(|(pat_seq, action, label)| {
                    let bindings = analysis::collect_bindings(pat_seq);
                    let action_expr = if let Some(a) = action {
                        quote!({ #a })
                    } else if bindings.is_empty() {
                        quote!(())
                    } else if bindings.len() == 1 {
                        let b = &bindings[0];
                        quote!(#b)
                    } else {
                        quote!(( #(#bindings),* ))
                    };
                    RuleVariant {
                        pattern: pat_seq.clone(), // Use clone() as it's Vec<ModelPattern>
                        label: label.clone(),
                        action: quote!({ #action_expr }),
                        with_span: false,
                        is_explicit: false,
                    }
                })
                .collect::<Vec<_>>();

            let variant_logic = generate_variants_internal(&temp_variants, false, ctx)?;

            let wrapped_logic = quote! {
                (|| -> syn::Result<_> {
                    #variant_logic
                })()
            };

            if let Some(bind) = binding {
                Ok(quote! {
                    let #bind = { #wrapped_logic }?;
                })
            } else {
                let group_bindings = analysis::collect_bindings(std::slice::from_ref(pattern));
                if group_bindings.is_empty() {
                    Ok(quote! { { #wrapped_logic }?; })
                } else {
                    let tuple_pat = quote!(( #(#group_bindings),* ));
                    Ok(quote! {
                        let #tuple_pat = { #wrapped_logic }?;
                    })
                }
            }
        }

        ModelPattern::Bracketed(s, _)
        | ModelPattern::Braced(s, _)
        | ModelPattern::Parenthesized(s, _) => {
            let delimiter = match pattern {
                ModelPattern::Bracketed(_, _) => '[',
                ModelPattern::Braced(_, _) => '{',
                _ => '(',
            };

            let inner_logic = generate_sequence_steps(s, ctx)?;
            let bindings = analysis::collect_bindings(s);

            let return_expr = if bindings.is_empty() {
                quote! { () }
            } else if bindings.len() == 1 {
                let b = &bindings[0];
                quote! { #b }
            } else {
                let b = &bindings;
                quote! { (#(#b),*) }
            };

            let parser_closure = quote! {
                |mut input, ctx| {
                    #inner_logic
                    Ok(#return_expr)
                }
            };

            let call_expr = quote! {
                rt::parse_delimited(
                    input,
                    ctx,
                    #parser_closure,
                    #delimiter
                )?
            };

            if bindings.is_empty() {
                Ok(quote! { let _ = #call_expr; })
            } else if bindings.len() == 1 {
                let bind = &bindings[0];
                Ok(quote! { let #bind = #call_expr; })
            } else {
                Ok(quote! { let (#(#bindings),*) = #call_expr; })
            }
        }

        ModelPattern::LexicalScope(inner, _) => {
            let inner_logic = generate_pattern_step(inner, ctx)?;
            Ok(quote! {
                ctx.enter_lexical();
                let _res = (|| -> syn::Result<_> {
                    #inner_logic
                    Ok(())
                })();
                ctx.exit_mode();
                _res?;
            })
        }
        ModelPattern::SpacedScope(inner, _) => {
            let inner_logic = generate_pattern_step(inner, ctx)?;
            Ok(quote! {
                ctx.enter_spaced();
                let _res = (|| -> syn::Result<_> {
                    #inner_logic
                    Ok(())
                })();
                ctx.exit_mode();
                _res?;
            })
        }

        ModelPattern::SpanBinding(inner, span_var, _) => {
            let (inner_pat, binding_name) = match &**inner {
                ModelPattern::RuleCall {
                    binding,
                    rule_path,
                    generics,
                    args,
                } => {
                    if let Some(b) = binding {
                        (inner.clone(), b.clone())
                    } else {
                        let temp = format_ident!("_val_{}", span_var);
                        let new_inner = ModelPattern::RuleCall {
                            binding: Some(temp.clone()),
                            rule_path: rule_path.clone(),
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

            let inner_code = generate_pattern_step(&inner_pat, ctx)?;

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
                        rule_path,
                        generics,
                        args,
                    } => Box::new(ModelPattern::RuleCall {
                        binding: Some(bind.clone()),
                        rule_path: rule_path.clone(),
                        generics: generics.clone(),
                        args: args.clone(),
                    }),
                    _ => body.clone(), // fallback
                }
            } else {
                body.clone()
            };

            let inner_logic = generate_pattern_step(&effective_body, ctx)?;
            let sync_peek =
                analysis::get_simple_peek(sync, ctx.custom_keywords)?.ok_or_else(|| {
                    syn::Error::new(
                        sync.span(),
                        "Sync pattern in recover(...) must have a simple start token.",
                    )
                })?;

            let bindings = analysis::collect_bindings(std::slice::from_ref(&effective_body));
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
            let inner_logic = generate_pattern_step(inner, ctx)?;

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
            let inner_logic = generate_pattern_step(inner, ctx)?;
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
            let inner_logic = generate_pattern_step(pattern, ctx)?;

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

        ModelPattern::Count {
            binding,
            pattern: inner,
            ..
        } => {
            // Note: bindings inside count are ignored as they are local to the loop/execution
            // and we return usize.
            // However, we must ensure we don't try to bind them in the outer scope.
            // Since we updated collect_bindings to return empty for Count, the outer scope won't expect them.
            // But generate_pattern_step(inner) WILL generate `let x = ...`.
            // So we must wrap inner_logic in a block where those bindings are allowed to drop.

            let count_logic = if let ModelPattern::Repeat(rep_inner, _) = &**inner {
                let inner_logic = generate_pattern_step(rep_inner, ctx)?;
                let peek_opt = analysis::get_simple_peek(rep_inner, ctx.custom_keywords)
                    .ok()
                    .flatten();

                let loop_body = if let Some(peek) = peek_opt {
                    quote! {
                        while input.peek(#peek) {
                            {
                                #inner_logic
                            }
                            _count += 1;
                        }
                    }
                } else {
                    let inner_bindings =
                        analysis::collect_bindings(std::slice::from_ref(rep_inner));
                    let return_tuple = if inner_bindings.is_empty() {
                        quote!(())
                    } else {
                        quote!(( #(#inner_bindings),* ))
                    };

                    quote! {
                       while let Some(_) = rt::attempt(input, ctx, |mut input, ctx| {
                           #inner_logic
                           Ok(#return_tuple)
                       })? {
                           _count += 1;
                       }
                    }
                };

                quote! {
                    let mut _count: usize = 0;
                    #loop_body
                    _count
                }
            } else if let ModelPattern::Plus(plus_inner, _) = &**inner {
                let inner_logic = generate_pattern_step(plus_inner, ctx)?;
                let peek_opt = analysis::get_simple_peek(plus_inner, ctx.custom_keywords)
                    .ok()
                    .flatten();

                let loop_body = if let Some(peek) = peek_opt {
                    quote! {
                        {
                            #inner_logic
                        }
                        _count += 1;
                        while input.peek(#peek) {
                            {
                                #inner_logic
                            }
                            _count += 1;
                        }
                    }
                } else {
                    let inner_bindings =
                        analysis::collect_bindings(std::slice::from_ref(plus_inner));
                    let return_tuple = if inner_bindings.is_empty() {
                        quote!(())
                    } else {
                        quote!(( #(#inner_bindings),* ))
                    };

                    quote! {
                       {
                           #inner_logic
                       }
                       _count += 1;
                       while let Some(_) = rt::attempt(input, ctx, |mut input, ctx| {
                           #inner_logic
                           Ok(#return_tuple)
                       })? {
                           _count += 1;
                       }
                    }
                };

                quote! {
                    let mut _count: usize = 0;
                    #loop_body
                    _count
                }
            } else if let ModelPattern::Optional(opt_inner, _) = &**inner {
                // For optional, we check if it matches.
                let inner_logic = generate_pattern_step(opt_inner, ctx)?;
                let peek_opt = analysis::get_simple_peek(opt_inner, ctx.custom_keywords)
                    .ok()
                    .flatten();
                let is_nullable = analysis::is_nullable(opt_inner);

                // If we have peek and it's not nullable, we can use peek.
                if let (Some(peek), false) = (peek_opt, is_nullable) {
                    quote! {
                        if input.peek(#peek) {
                            {
                                // Pass ctx to attempt just to be safe and consume tokens
                                let _ = rt::attempt(input, ctx, |mut input, ctx| { #inner_logic Ok(()) })?;
                            }
                            1usize
                        } else {
                            0usize
                        }
                    }
                } else {
                    // Generic attempt
                    quote! {
                        if let Some(_) = rt::attempt(input, ctx, |mut input, ctx| { #inner_logic Ok(()) })? {
                            1usize
                        } else {
                            0usize
                        }
                    }
                }
            } else {
                // Fallback: run the pattern. If success, return 1.
                let inner_logic = generate_pattern_step(inner, ctx)?;
                quote! {
                    {
                        #inner_logic
                        1usize
                    }
                }
            };

            if let Some(bind) = binding {
                Ok(quote! { let #bind = { #count_logic }; })
            } else {
                Ok(quote! { let _ = { #count_logic }; })
            }
        }

        ModelPattern::Fail { message, .. } => {
            let arg_expr = if let Some(Lit::Str(s)) = message {
                s.value()
            } else {
                "Explicit failure".to_string()
            };

            Ok(quote! {
                ctx.raise_failure::<()>(#arg_expr, input.span())?;
            })
        }
    }
}

fn generate_rule_call_expr(
    rule_path: &syn::Path,
    args: &[Argument],
    ctx: &CodegenContext,
) -> Result<TokenStream> {
    let mut arg_exprs: Vec<TokenStream> = Vec::new();

    for arg in args {
        match arg {
            Argument::Positional(p) | Argument::Named(_, p) => match p {
                ModelPattern::Lit { lit, .. } => arg_exprs.push(quote!(#lit)),
                ModelPattern::RuleCall {
                    rule_path, args, ..
                } if args.is_empty() && rule_path.get_ident().is_some() => {
                    let ident = rule_path.get_ident().unwrap();
                    arg_exprs.push(quote!(#ident));
                }
                _ => {
                    return Ok(quote!(compile_error!(
                        "Complex pattern used as runtime argument"
                    )));
                }
            },
        }
    }

    // 1. External Call (Imports or Namespaced paths)
    // If it has > 1 segment, or if it has 1 segment that matches an external rule or import alias (though alias matching might be implicit by path resolution).

    // Check if it's an extern rule (single ident)
    if let Some(ident) = rule_path.get_ident() {
        if ctx.grammar.extern_rules.iter().any(|er| er.name == *ident) {
            // Extern rule: call exactly as named, no ctx
            if arg_exprs.is_empty() {
                Ok(quote!(#rule_path(input)?))
            } else {
                Ok(quote!(#rule_path(input, #(#arg_exprs),*)?))
            }
        } else if ctx.grammar.rules.iter().any(|r| r.name == *ident) {
            // Local rule: call parse_{name}_impl with ctx
            let impl_name = format_ident!("parse_{}_impl", ident);
            if arg_exprs.is_empty() {
                Ok(quote!(#impl_name(&mut input, ctx)?))
            } else {
                Ok(quote!(#impl_name(&mut input, ctx, #(#arg_exprs),*)?))
            }
        } else {
            // Fallback: Treat as external function call (e.g. builtin or user-imported function)
            // Assume standard signature: func(input)
            if arg_exprs.is_empty() {
                Ok(quote!(#rule_path(input)?))
            } else {
                Ok(quote!(#rule_path(input, #(#arg_exprs),*)?))
            }
        }
    } else {
        // Multi-segment path
        // Apply parse_ prefix ONLY if first segment is a known import alias

        let first_seg = &rule_path.segments.first().unwrap().ident;
        let is_import_alias = ctx
            .grammar
            .imports
            .iter()
            .any(|imp| imp.alias == *first_seg);

        if is_import_alias {
            let mut new_path = rule_path.clone();
            let last_seg = new_path.segments.last_mut().unwrap();
            let last_ident = last_seg.ident.clone();
            
            // CRITICAL FIX: Immer die interne _impl Funktion nutzen und den shared 'ctx' übergeben
            let new_ident = format_ident!("parse_{}_impl", last_ident);
            last_seg.ident = new_ident;

            if arg_exprs.is_empty() {
                Ok(quote!(#new_path(&mut input, ctx)?))
            } else {
                Ok(quote!(#new_path(&mut input, ctx, #(#arg_exprs),*)?))
            }
        } else {
            // Standard call
            if arg_exprs.is_empty() {
                Ok(quote!(#rule_path(input)?))
            } else {
                Ok(quote!(#rule_path(input, #(#arg_exprs),*)?))
            }
        }
    }
}
