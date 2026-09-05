use crate::backend::SynBackend;
use crate::codegen::CodegenContext;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Lit, Result};
use syn_grammar_model::{analysis, model::*, Backend};

pub fn generate_sequence(
    patterns: &[ModelPattern],
    action: &TokenStream,
    ctx: &CodegenContext,
) -> Result<TokenStream> {
    let steps = generate_sequence_steps(patterns, ctx)?;
    Ok(quote! {
        #steps
        // The action block runs in its own syn::Result closure so that
        // user code inside it can still use `return Err(syn::Error::new(..))` and `?`
        // on syn results.
        let _action_res = (|| -> syn::Result<_> { Ok({ #action }) })()
            .map_err(rt::ParseError::from)?;
        Ok(_action_res)
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
                // On the stream, the token parser checks the joint spacing itself
                // and costs O(1) - the previously needed advance counting of the
                // source tokens (`take_fixed`) is gone.
                if token_types.len() <= 1 {
                    let parses = token_types.iter().map(|ty| {
                        let bind_stmt = if let Some(bind) = binding { quote!(let #bind = _t;) } else { quote!() };
                        quote! {
                            let _t = rt::parse_syn::<#ty>(input)?;
                            ctx.record_span(syn::spanned::Spanned::span(&_t)).map_err(|e| rt::ParseError::new(syn::spanned::Spanned::span(&_t), e.to_string()))?;
                            #bind_stmt
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
                            let #var = rt::parse_syn::<#ty>(input)?;
                        });
                        results.push(var.clone());
                        if i == token_types.len() - 1 {
                            steps.push(quote! { ctx.record_span(syn::spanned::Spanned::span(&#var)).map_err(|e| rt::ParseError::new(syn::spanned::Spanned::span(&#var), e.to_string()))?; });
                        }
                        if i > 0 {
                            let prev = format_ident!("_t{}", i - 1);
                            let err_msg =
                                format!("expected '{}', found space between tokens", lit.value());
                            checks.push(quote! {
                                if syn::spanned::Spanned::span(&#prev).end() != syn::spanned::Spanned::span(&#var).start() {
                                    return Err(rt::ParseError::new(syn::spanned::Spanned::span(&#var), #err_msg));
                                }
                            });
                        }
                    }
                    let bind_stmt = if let Some(bind) = binding {
                        quote! { let #bind = ( #(#results),* ); }
                    } else {
                        quote! {}
                    };
                    Ok(quote! { #(#steps)* #(#checks)* #bind_stmt })
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
            let is_builtin = rule_name_ident
                .is_some_and(|ident| SynBackend::get_builtins().iter().any(|b| ident == b.name));
            let is_syn_type = rule_path
                .segments
                .first()
                .is_some_and(|seg| seg.ident == "syn");

            if is_syn_type {
                let bind_stmt = if let Some(bind) = binding {
                    quote!(let #bind = _val;)
                } else {
                    quote!()
                };
                // Since ADR 15, stage 3, this is an ordinary `parse` call
                // on the existing stream: O(length of the type). The special paths
                // from stage 1 (`take_braced_block`, `take_upto_group`) are thus
                // moot - they bypassed exactly the materialization that
                // no longer exists at all.
                Ok(quote! {
                    let _val = rt::parse_syn::<#rule_path>(input)?;
                    #bind_stmt
                })
            } else if rule_path.is_ident("separated") {
                let rule_arg = match &args[0] {
                    Argument::Positional(p) | Argument::Named(_, p) => p,
                };
                let sep_arg = match &args[1] {
                    Argument::Positional(p) | Argument::Named(_, p) => p,
                };
                let mut min = 0usize;
                let mut trailing = false;
                let mut item_label: Option<String> = None;

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
                            } else if id == "item_label" {
                                if let ModelPattern::Lit {
                                    lit: Lit::Str(s), ..
                                } = val
                                {
                                    item_label = Some(s.value());
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

                let container_ty = generics.first().map_or(quote!(Vec), |ty| match ty {
                    syn::Type::Infer(_) => quote!(Vec),
                    _ => quote!(#ty),
                });
                let (rule_arg_with_binding, item_binding) = match rule_arg {
                    ModelPattern::RuleCall {
                        binding: None,
                        rule_path,
                        generics,
                        args,
                    } => {
                        let temp = format_ident!("_item");
                        (
                            ModelPattern::RuleCall {
                                binding: Some(temp.clone()),
                                rule_path: rule_path.clone(),
                                generics: generics.clone(),
                                args: args.clone(),
                            },
                            vec![temp],
                        )
                    }
                    ModelPattern::Lit { binding: None, lit } => {
                        let temp = format_ident!("_item");
                        (
                            ModelPattern::Lit {
                                binding: Some(temp.clone()),
                                lit: lit.clone(),
                            },
                            vec![temp],
                        )
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
                    quote!(#b)
                } else if item_binding.is_empty() {
                    quote!(())
                } else {
                    quote!((#(#item_binding),*))
                };
                let label_tokens = if let Some(l) = item_label {
                    quote!(#l)
                } else {
                    quote!("item")
                };

                let bind_stmt = if let Some(bind) = binding {
                    quote!(let #bind = #container_ty::from_iter(_items_vec);)
                } else {
                    quote!()
                };

                Ok(quote! {
                    let _items_vec = rt::parse_separated(
                        input, ctx,
                        |input: &rt::Stream<'a>, ctx: &mut rt::ParseContext<'a>| { #rule_parser Ok(#item_return_expr) },
                        |input: &rt::Stream<'a>, ctx: &mut rt::ParseContext<'a>| { #sep_parser Ok(()) },
                        #min, #trailing, #label_tokens
                    )?;
                    #bind_stmt
                })
            } else if rule_path.is_ident("repeated") {
                let rule_arg = match &args[0] {
                    Argument::Positional(p) | Argument::Named(_, p) => p,
                };
                let mut min = 0usize;
                let mut item_label: Option<String> = None;

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
                            } else if id == "item_label" {
                                if let ModelPattern::Lit {
                                    lit: Lit::Str(s), ..
                                } = val
                                {
                                    item_label = Some(s.value());
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

                let container_ty = generics.first().map_or(quote!(Vec), |ty| match ty {
                    syn::Type::Infer(_) => quote!(Vec),
                    _ => quote!(#ty),
                });
                let (rule_arg_with_binding, item_binding) = match rule_arg {
                    ModelPattern::RuleCall {
                        binding: None,
                        rule_path,
                        generics,
                        args,
                    } => {
                        let temp = format_ident!("_item");
                        (
                            ModelPattern::RuleCall {
                                binding: Some(temp.clone()),
                                rule_path: rule_path.clone(),
                                generics: generics.clone(),
                                args: args.clone(),
                            },
                            vec![temp],
                        )
                    }
                    ModelPattern::Lit { binding: None, lit } => {
                        let temp = format_ident!("_item");
                        (
                            ModelPattern::Lit {
                                binding: Some(temp.clone()),
                                lit: lit.clone(),
                            },
                            vec![temp],
                        )
                    }
                    _ => (
                        rule_arg.clone(),
                        analysis::collect_bindings(std::slice::from_ref(rule_arg)),
                    ),
                };

                let rule_parser = generate_pattern_step(&rule_arg_with_binding, ctx)?;
                let item_return_expr = if item_binding.len() == 1 {
                    let b = &item_binding[0];
                    quote!(#b)
                } else if item_binding.is_empty() {
                    quote!(())
                } else {
                    quote!((#(#item_binding),*))
                };
                let item_label_expr = if let Some(msg) = item_label {
                    quote!(#msg)
                } else {
                    quote!("item")
                };

                let bind_stmt = if let Some(bind) = binding {
                    quote!(let #bind = #container_ty::from_iter(_items_vec);)
                } else {
                    quote!()
                };

                Ok(quote! {
                    let _items_vec = rt::parse_repeated(
                        input, ctx,
                        |input: &rt::Stream<'a>, ctx: &mut rt::ParseContext<'a>| { #rule_parser Ok(#item_return_expr) },
                        #min, #item_label_expr
                    )?;
                    #bind_stmt
                })
            } else if is_builtin {
                let rule_name_str = rule_name_ident.unwrap().to_string();
                let expr = match rule_name_str.as_str() {
                    // The character filters and `any_byte` remain cursor primitives
                    // (O(1), no stream needed); `rt::step` lets them run in a
                    // `step` episode and advances the stream accordingly.
                    "alpha" | "digit" | "alphanumeric" | "hex_digit" | "oct_digit" => {
                        let func = format_ident!("{}", rule_name_str);
                        quote! { rt::step(input, rt::token_filter::#func)? }
                    }
                    "eof" => {
                        return Ok(quote! {
                            if !input.cursor().eof() { return Err(rt::ParseError::at_cursor(input.cursor(), "expected end of input")); }
                        })
                    }
                    "whitespace" => {
                        return Ok(quote! {
                            if !ctx.check_whitespace(input.cursor().span()) { return Err(rt::ParseError::at_cursor(input.cursor(), "expected whitespace")); }
                        })
                    }
                    "any_byte" => quote! { rt::step(input, rt::take_single::<syn::LitByte>)? },
                    _ => {
                        let impl_name = format_ident!("parse_{}_impl", rule_name_ident.unwrap());
                        quote! { rt::builtins::#impl_name(input, ctx)? }
                    }
                };

                let bind_stmt = if let Some(bind) = binding {
                    quote!(let #bind = _val;)
                } else {
                    quote!()
                };
                Ok(quote! {
                    let _val = #expr;
                    #bind_stmt
                })
            } else {
                let func_call = generate_rule_call_expr(rule_path, args, ctx)?;
                let bind_stmt = if let Some(bind) = binding {
                    quote!(let #bind = _val;)
                } else {
                    quote!()
                };
                Ok(quote! {
                    let _val = #func_call;
                    #bind_stmt
                })
            }
        }
        ModelPattern::Repeat(inner, _) => {
            let bindings = analysis::collect_bindings(std::slice::from_ref(inner));
            let inner_logic = generate_pattern_step(inner, ctx)?;

            if !bindings.is_empty() {
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
                let return_tuple = quote!(( #(#bindings),* ));
                let tuple_pat = quote!(( #(#bindings),* ));

                Ok(quote! {
                    #(#init_vecs)*
                    loop {
                        let _start_cursor = input.cursor();
                        let _fork = rt::fork(input);
                        let _res = (|| -> rt::StreamResult<'a, _> {
                            let input = &_fork;
                            #inner_logic
                            Ok(#return_tuple)
                        })();
                        match _res {
                            Ok(vals) => {
                                // Zero-progress guard (see above).
                                if _fork.cursor() == _start_cursor { break; }
                                rt::advance_to(input, &_fork);
                                let #tuple_pat = vals;
                                #(#push_vecs)*
                            }
                            Err(e) => {
                                if e.priority >= 50 { return Err(e); }
                                // The repetition ends regularly - the reason is
                                // recorded, otherwise it would be lost here.
                                ctx.record_failure(&e);
                                break;
                            }
                        }
                    }
                    #(#finalize_vecs)*
                })
            } else {
                Ok(quote! {
                    loop {
                        let _start_cursor = input.cursor();
                        let _fork = rt::fork(input);
                        let _res = (|| -> rt::StreamResult<'a, _> {
                            let input = &_fork;
                            #inner_logic
                            Ok(())
                        })();
                        match _res {
                            Ok(()) => {
                                // Zero-progress guard: otherwise the loop spins forever
                                // if the inner pattern matches without consuming tokens.
                                if _fork.cursor() == _start_cursor { break; }
                                rt::advance_to(input, &_fork);
                            }
                            Err(e) => {
                                if e.priority >= 50 { return Err(e); }
                                // The repetition ends regularly - the reason is
                                // recorded, otherwise it would be lost here.
                                ctx.record_failure(&e);
                                break;
                            }
                        }
                    }
                })
            }
        }
        ModelPattern::Plus(inner, _) => {
            let bindings = analysis::collect_bindings(std::slice::from_ref(inner));
            let inner_logic = generate_pattern_step(inner, ctx)?;

            if !bindings.is_empty() {
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
                let return_tuple = quote!(( #(#bindings),* ));
                let tuple_pat = quote!(( #(#bindings),* ));

                Ok(quote! {
                    #(#init_vecs)*
                    // Do NOT wrap the first mandatory item in a block: #inner_logic
                    // ends with `let mut cursor = next_cursor;`, the advance would otherwise
                    // be lost at the end of the block and the loop would run over it again.
                    #inner_logic
                    let #tuple_pat = #return_tuple;
                    #(#push_vecs)*
                    loop {
                        let _start_cursor = input.cursor();
                        let _fork = rt::fork(input);
                        let _res = (|| -> rt::StreamResult<'a, _> {
                            let input = &_fork;
                            #inner_logic
                            Ok(#return_tuple)
                        })();
                        match _res {
                            Ok(vals) => {
                                // Zero-progress guard (see above).
                                if _fork.cursor() == _start_cursor { break; }
                                rt::advance_to(input, &_fork);
                                let #tuple_pat = vals;
                                #(#push_vecs)*
                            }
                            Err(e) => {
                                if e.priority >= 50 { return Err(e); }
                                // The repetition ends regularly - the reason is
                                // recorded, otherwise it would be lost here.
                                ctx.record_failure(&e);
                                break;
                            }
                        }
                    }
                    #(#finalize_vecs)*
                })
            } else {
                Ok(quote! {
                    #inner_logic
                    loop {
                        let _start_cursor = input.cursor();
                        let _fork = rt::fork(input);
                        let _res = (|| -> rt::StreamResult<'a, _> {
                            let input = &_fork;
                            #inner_logic
                            Ok(())
                        })();
                        match _res {
                            Ok(()) => {
                                // Zero-progress guard: otherwise the loop spins forever
                                // if the inner pattern matches without consuming tokens.
                                if _fork.cursor() == _start_cursor { break; }
                                rt::advance_to(input, &_fork);
                            }
                            Err(e) => {
                                if e.priority >= 50 { return Err(e); }
                                // The repetition ends regularly - the reason is
                                // recorded, otherwise it would be lost here.
                                ctx.record_failure(&e);
                                break;
                            }
                        }
                    }
                })
            }
        }
        ModelPattern::Optional(inner, _) => {
            let inner_logic = generate_pattern_step(inner, ctx)?;
            let bindings = analysis::collect_bindings(std::slice::from_ref(inner));

            if bindings.is_empty() {
                Ok(quote! {
                    let _fork = rt::fork(input);
                    let _opt_res = (|| -> rt::StreamResult<'a, _> {
                        let input = &_fork;
                        #inner_logic
                        Ok(())
                    })();
                    match _opt_res {
                        Ok(()) => { rt::advance_to(input, &_fork); }
                        Err(e) => {
                            if e.priority >= 50 { return Err(e); }
                            ctx.record_failure(&e);
                        }
                    }
                })
            } else {
                let vars: Vec<_> = bindings.iter().map(|b| quote!(#b)).collect();
                let some_vars: Vec<_> = bindings.iter().map(|b| quote!(Some(#b))).collect();
                let none_vars: Vec<_> = bindings.iter().map(|_| quote!(None)).collect();
                Ok(quote! {
                    let _fork = rt::fork(input);
                    let _opt_res = (|| -> rt::StreamResult<'a, _> {
                        let input = &_fork;
                        #inner_logic
                        Ok((#(#vars),*))
                    })();
                    let (#(#vars),*) = match _opt_res {
                        Ok(vals) => {
                            rt::advance_to(input, &_fork);
                            let (#(#vars),*) = vals;
                            (#(#some_vars),*)
                        }
                        Err(e) => {
                            if e.priority >= 50 { return Err(e); }
                            ctx.record_failure(&e);
                            (#(#none_vars),*)
                        }
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
                        pattern: pat_seq.clone(),
                        label: label.clone(),
                        action: quote!({ #action_expr }),
                        with_span: false,
                        is_explicit: false,
                    }
                })
                .collect::<Vec<_>>();

            let variant_logic = generate_variants_internal(&temp_variants, false, ctx)?;
            let bind_stmt = if let Some(bind) = binding {
                quote!(let #bind = _val;)
            } else {
                let b = analysis::collect_bindings(std::slice::from_ref(pattern));
                if b.is_empty() {
                    quote!()
                } else {
                    quote!(let (#(#b),*) = _val;)
                }
            };

            Ok(quote! {
                let _val = (|| -> rt::StreamResult<'a, _> { #variant_logic })()?;
                #bind_stmt
            })
        }
        ModelPattern::Bracketed(s, _)
        | ModelPattern::Braced(s, _)
        | ModelPattern::Parenthesized(s, _) => {
            let delimiter = match pattern {
                ModelPattern::Bracketed(_, _) => quote!(Bracket),
                ModelPattern::Braced(_, _) => quote!(Brace),
                _ => quote!(Parenthesis),
            };
            let inner_logic = generate_sequence_steps(s, ctx)?;
            let bindings = analysis::collect_bindings(s);
            let return_expr = if bindings.is_empty() {
                quote!(())
            } else if bindings.len() == 1 {
                let b = &bindings[0];
                quote!(#b)
            } else {
                quote!((#(#bindings),*))
            };
            let bind_stmt = if bindings.is_empty() {
                quote!()
            } else if bindings.len() == 1 {
                let bind = &bindings[0];
                quote!(let #bind = _val;)
            } else {
                quote!(let (#(#bindings),*) = _val;)
            };

            // The bindings of the group must land in the ENCLOSING scope, otherwise
            // the action block does not see them (they used to die with the if-let block).
            Ok(quote! {
                // `rt::group` descends into the delimiters and advances the outer
                // stream past the group. The content is a separate stream with
                // the same token lifetime - only because of that can errors from the
                // group be carried outward.
                let (_span, _content) = rt::group(input, proc_macro2::Delimiter::#delimiter)?;
                // Inside the group, Cursor::eof() reports the end of the group, not
                // the end of input. Record the depth so that messages can name the
                // difference.
                ctx.enter_group();
                let _grp_res = (|| -> rt::StreamResult<'a, _> {
                    let input = &_content;
                    #inner_logic
                    Ok(#return_expr)
                })();
                ctx.exit_group();
                let _val = _grp_res?;
                // "unexpected token in delimited group" is only a placeholder for
                // "the content was not consumed completely". If a reason was recorded
                // along the way, it is strictly more informative - hence
                // not structural, so that this one does not win.
                if !_content.cursor().eof() {
                    return Err(ctx.best_error(
                        rt::ParseError::at_cursor(_content.cursor(), "unexpected token in delimited group")
                    ));
                }
                #bind_stmt
            })
        }
        ModelPattern::LexicalScope(inner, _) => {
            let inner_logic = generate_pattern_step(inner, ctx)?;
            Ok(quote! {
                ctx.enter_lexical();
                // The `?` MUST come after `exit_mode()` - otherwise the
                // mode stays on the stack in case of an error. The delimiter branch
                // does the same with `exit_group()`.
                let _mode_res = (|| -> rt::StreamResult<'a, _> {
                    #inner_logic
                    Ok(())
                })();
                ctx.exit_mode();
                _mode_res?;
            })
        }
        ModelPattern::SpacedScope(inner, _) => {
            let inner_logic = generate_pattern_step(inner, ctx)?;
            Ok(quote! {
                ctx.enter_spaced();
                // The `?` MUST come after `exit_mode()` - otherwise the
                // mode stays on the stack in case of an error. The delimiter branch
                // does the same with `exit_group()`.
                let _mode_res = (|| -> rt::StreamResult<'a, _> {
                    #inner_logic
                    Ok(())
                })();
                ctx.exit_mode();
                _mode_res?;
            })
        }
        ModelPattern::SpanBinding(inner, span_var, _) => {
            let inner_logic = generate_pattern_step(inner, ctx)?;
            Ok(quote! {
                let _span_start = input.cursor().span();
                #inner_logic
                let _span_end = input.cursor().span();
                let #span_var = _span_start.join(_span_end).unwrap_or(_span_start);
            })
        }
        ModelPattern::Recover {
            binding,
            body,
            sync,
            span: _,
        } => {
            // An unbound rule call in the body inherits the outer binding name,
            // otherwise the body would have no value and recover() would yield () instead of Option<T>.
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
                    _ => body.clone(),
                }
            } else {
                body.clone()
            };
            let inner_logic = generate_pattern_step(&effective_body, ctx)?;
            let sync_peek = analysis::get_simple_peek(sync, ctx.custom_keywords)?.unwrap();
            let bindings = analysis::collect_bindings(std::slice::from_ref(&effective_body));

            let return_expr = if bindings.is_empty() {
                quote!(())
            } else if bindings.len() == 1 {
                let b = &bindings[0];
                quote!(#b)
            } else {
                quote!((#(#bindings),*))
            };
            let bind_stmt = if let Some(bind) = binding {
                quote!(let #bind = _val;)
            } else if bindings.is_empty() {
                quote!()
            } else {
                quote!(let (#(#bindings),*) = _val;)
            };

            let none_exprs = bindings.iter().map(|_| quote!(Option::<_>::None));
            let some_exprs = bindings.iter().map(|b| quote!(Some(#b)));

            // No binding: nothing to assign. Exactly one: _val is wrapped directly in
            // Some(..), an intermediate assignment would move the value away
            // beforehand. Only from two on is it destructured.
            let some_assign = if bindings.len() <= 1 {
                quote!()
            } else {
                quote!(let (#(#bindings),*) = _val;)
            };
            let option_ret = if bindings.is_empty() {
                quote!(())
            } else if bindings.len() == 1 {
                quote!(Some(_val))
            } else {
                quote!((#(#some_exprs),*))
            };
            let none_ret = if bindings.is_empty() {
                quote!(())
            } else if bindings.len() == 1 {
                quote!(None)
            } else {
                quote!((#(#none_exprs),*))
            };

            Ok(quote! {
                let _fork = rt::fork(input);
                let _rec_res = (|| -> rt::StreamResult<'a, _> {
                    let input = &_fork;
                    #inner_logic
                    Ok(#return_expr)
                })();
                let _val = match _rec_res {
                    Ok(_val) => {
                        rt::advance_to(input, &_fork);
                        #some_assign
                        #option_ret
                    }
                    Err(e) => {
                        if e.priority >= 50 { return Err(e); }
                        // The fork was NOT adopted - the stream is still at
                        // the start of the body. Skip from there up to the next
                        // synchronization mark.
                        loop {
                            if input.cursor().eof() { break; }
                            if rt::peek_syn(input.cursor(), #sync_peek) { break; }
                            if rt::take_token(input).is_err() { break; }
                        }
                        #none_ret
                    }
                };
                #bind_stmt
            })
        }
        ModelPattern::Peek(inner, _) => {
            let inner_logic = generate_pattern_step(inner, ctx)?;
            let bindings = analysis::collect_bindings(std::slice::from_ref(inner));
            let return_expr = if bindings.is_empty() {
                quote!(())
            } else if bindings.len() == 1 {
                let b = &bindings[0];
                quote!(#b)
            } else {
                quote!((#(#bindings),*))
            };
            let bind_stmt = if bindings.is_empty() {
                quote!()
            } else if bindings.len() == 1 {
                let bind = &bindings[0];
                quote!(let #bind = _val;)
            } else {
                quote!(let (#(#bindings),*) = _val;)
            };

            Ok(quote! {
                // Lookahead: runs on a fork that is never adopted -
                // so the stream stays where it is.
                let _val = (|| -> rt::StreamResult<'a, _> {
                    let _fork = rt::fork(input);
                    let input = &_fork;
                    #inner_logic
                    Ok(#return_expr)
                })()?;
                #bind_stmt
            })
        }
        ModelPattern::Not(inner, _) => {
            let inner_logic = generate_pattern_step(inner, ctx)?;
            // If `not(..)` rejects a rule, its name belongs in the message -
            // otherwise it only says "unexpected match" without any clue.
            let inner_name = match &**inner {
                ModelPattern::RuleCall { rule_path, .. } => rule_path
                    .segments
                    .last()
                    .map(|s| s.ident.to_string().replace('_', " ")),
                _ => None,
            };
            let current_rule = ctx.current_rule.clone();
            let message = match inner_name {
                Some(name) => quote! {
                    {
                        let _found = input.cursor()
                            .token_tree()
                            .map(|(tt, _)| tt.to_string())
                            .unwrap_or_default();
                        format!(
                            "unexpected match for rule `{}`; found `{}` in rule `{}`",
                            #name, _found, #current_rule
                        )
                    }
                },
                None => quote!("unexpected match".to_string()),
            };
            Ok(quote! {
                // As with lookahead: a fork that is never adopted.
                let _not_res = (|| -> rt::StreamResult<'a, _> {
                    let _fork = rt::fork(input);
                    let input = &_fork;
                    #inner_logic
                    Ok(())
                })();
                if _not_res.is_ok() {
                    return Err(rt::ParseError::at_cursor(input.cursor(), #message).with_priority(50));
                }
            })
        }
        ModelPattern::Until {
            binding, pattern, ..
        } => {
            let inner_logic = generate_pattern_step(pattern, ctx)?;
            let bind_stmt = if let Some(bind) = binding {
                quote!(let #bind = _tokens_stream;)
            } else {
                quote!()
            };
            Ok(quote! {
                let mut _tokens = Vec::new();
                loop {
                    if input.cursor().eof() { break; }
                    let _is_match = (|| -> rt::StreamResult<'a, _> {
                        let _fork = rt::fork(input);
                        let input = &_fork;
                        #inner_logic
                        Ok(())
                    })().is_ok();
                    if _is_match { break; }
                    match rt::take_token(input) {
                        Ok(tt) => _tokens.push(tt),
                        Err(_) => break,
                    }
                }
                let _tokens_stream = proc_macro2::TokenStream::from_iter(_tokens);
                #bind_stmt
            })
        }
        ModelPattern::Count {
            binding,
            pattern: inner,
            ..
        } => {
            // count(..) counts the ITEM, not the repetition operator.
            // `count("a"*)` on "a a a" is 3, not 1: so the operator must be
            // stripped off and its item counted. A generic
            // loop over "a"* would consume everything on the first pass
            // and then run on empty forever afterwards.
            let bind_stmt = if let Some(bind) = binding {
                quote!(let #bind = _count;)
            } else {
                quote!(let _ = _count;)
            };

            // One loop pass over the item, with zero-progress guard.
            let loop_over = |elem_logic: &proc_macro2::TokenStream| {
                quote! {
                    loop {
                        let _start_cursor = input.cursor();
                        let _fork = rt::fork(input);
                        let _res = (|| -> rt::StreamResult<'a, _> {
                            let input = &_fork;
                            #elem_logic
                            Ok(())
                        })();
                        match _res {
                            Ok(()) => {
                                if _fork.cursor() == _start_cursor { break; }
                                rt::advance_to(input, &_fork);
                                _count += 1;
                            }
                            Err(e) => {
                                if e.priority >= 50 { return Err(e); }
                                // The repetition ends regularly - the reason is
                                // recorded, otherwise it would be lost here.
                                ctx.record_failure(&e);
                                break;
                            }
                        }
                    }
                }
            };

            let count_logic = match &**inner {
                ModelPattern::Repeat(elem, _) => {
                    let elem_logic = generate_pattern_step(elem, ctx)?;
                    let lp = loop_over(&elem_logic);
                    quote! { let mut _count: usize = 0; #lp }
                }
                ModelPattern::Plus(elem, _) => {
                    let elem_logic = generate_pattern_step(elem, ctx)?;
                    let lp = loop_over(&elem_logic);
                    // The first item is mandatory: do not wrap it in a block,
                    // otherwise the cursor advance is lost at the end of the block.
                    quote! {
                        let mut _count: usize = 0;
                        #elem_logic
                        _count += 1;
                        #lp
                    }
                }
                ModelPattern::Optional(elem, _) => {
                    let elem_logic = generate_pattern_step(elem, ctx)?;
                    quote! {
                        let mut _count: usize = 0;
                        let _fork = rt::fork(input);
                        let _res = (|| -> rt::StreamResult<'a, _> {
                            let input = &_fork;
                            #elem_logic
                            Ok(())
                        })();
                        match _res {
                            Ok(()) => { rt::advance_to(input, &_fork); _count += 1; }
                            Err(e) => {
                            if e.priority >= 50 { return Err(e); }
                            ctx.record_failure(&e);
                        }
                        }
                    }
                }
                _ => {
                    let elem_logic = generate_pattern_step(inner, ctx)?;
                    quote! {
                        let mut _count: usize = 0;
                        #elem_logic
                        _count += 1;
                    }
                }
            };

            Ok(quote! {
                #count_logic
                #bind_stmt
            })
        }
        ModelPattern::Fail { message, .. } => {
            let arg_expr = if let Some(Lit::Str(s)) = message {
                s.value()
            } else {
                "Explicit failure".to_string()
            };
            Ok(
                quote! { return Err(rt::ParseError::at_cursor(input.cursor(), #arg_expr).with_priority(50)); },
            )
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
                    )))
                }
            },
        }
    }

    if let Some(ident) = rule_path.get_ident() {
        if ctx.grammar.extern_rules.iter().any(|er| er.name == *ident) {
            Ok(if arg_exprs.is_empty() {
                quote!(#rule_path(input)?)
            } else {
                quote!(#rule_path(input, #(#arg_exprs),*)?)
            })
        } else if ctx.grammar.rules.iter().any(|r| r.name == *ident) {
            let impl_name = format_ident!("parse_{}_impl", ident);
            Ok(if arg_exprs.is_empty() {
                quote!(#impl_name(input, ctx)?)
            } else {
                quote!(#impl_name(input, ctx, #(#arg_exprs),*)?)
            })
        } else {
            Ok(if arg_exprs.is_empty() {
                quote!(#rule_path(input)?)
            } else {
                quote!(#rule_path(input, #(#arg_exprs),*)?)
            })
        }
    } else {
        let first_seg = &rule_path.segments.first().unwrap().ident;
        let is_import_alias = ctx
            .grammar
            .imports
            .iter()
            .any(|imp| imp.alias == *first_seg);
        if is_import_alias {
            let mut new_path = rule_path.clone();
            let last_seg = new_path.segments.last_mut().unwrap();
            last_seg.ident = format_ident!("parse_{}_impl", last_seg.ident);
            Ok(if arg_exprs.is_empty() {
                quote!(#new_path(input, ctx)?)
            } else {
                quote!(#new_path(input, ctx, #(#arg_exprs),*)?)
            })
        } else {
            Ok(if arg_exprs.is_empty() {
                quote!(#rule_path(input)?)
            } else {
                quote!(#rule_path(input, #(#arg_exprs),*)?)
            })
        }
    }
}
