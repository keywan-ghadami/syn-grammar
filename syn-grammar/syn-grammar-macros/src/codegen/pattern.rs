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
        // Der Action-Block laeuft in einer eigenen syn::Result-Closure, damit
        // Nutzercode darin weiterhin `return Err(syn::Error::new(..))` und `?`
        // auf syn-Ergebnisse verwenden kann.
        let _action_res = (|| -> syn::Result<_> { Ok({ #action }) })()
            .map_err(rt::ParseError::from)?;
        Ok((_action_res, cursor))
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
                        let bind_stmt = if let Some(bind) = binding { quote!(let #bind = _t;) } else { quote!() };
                        quote! {
                            let (_t, next_cursor) = rt::invoke_syn_parser::<#ty>(cursor)?;
                            ctx.record_span(syn::spanned::Spanned::span(&_t)).map_err(|e| rt::ParseError::new(syn::spanned::Spanned::span(&_t), e.to_string()))?;
                            #bind_stmt
                            let mut cursor = next_cursor;
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
                            let (#var, next_cursor) = rt::invoke_syn_parser::<#ty>(cursor)?;
                            let mut cursor = next_cursor;
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
                Ok(quote! {
                    let (_val, next_cursor) = rt::invoke_syn_parser::<#rule_path>(cursor)?;
                    #bind_stmt
                    let mut cursor = next_cursor;
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
                    let (_items_vec, next_cursor) = rt::parse_separated(
                        cursor, ctx,
                        |mut cursor, ctx| { #rule_parser Ok((#item_return_expr, cursor)) },
                        |mut cursor, ctx| { #sep_parser Ok(((), cursor)) },
                        #min, #trailing, #label_tokens
                    )?;
                    #bind_stmt
                    let mut cursor = next_cursor;
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
                    let (_items_vec, next_cursor) = rt::parse_repeated(
                        cursor, ctx,
                        |mut cursor, ctx| { #rule_parser Ok((#item_return_expr, cursor)) },
                        #min, #item_label_expr
                    )?;
                    #bind_stmt
                    let mut cursor = next_cursor;
                })
            } else if is_builtin {
                let rule_name_str = rule_name_ident.unwrap().to_string();
                let expr = match rule_name_str.as_str() {
                    "alpha" | "digit" | "alphanumeric" | "hex_digit" | "oct_digit" => {
                        let func = format_ident!("{}", rule_name_str);
                        quote! { rt::token_filter::#func(cursor)? }
                    }
                    "eof" => {
                        return Ok(quote! {
                            if !cursor.eof() { return Err(rt::ParseError::at_cursor(cursor, "expected end of input")); }
                        })
                    }
                    "whitespace" => {
                        return Ok(quote! {
                            if !ctx.check_whitespace(cursor.span()) { return Err(rt::ParseError::at_cursor(cursor, "expected whitespace")); }
                        })
                    }
                    "any_byte" => quote! { rt::invoke_syn_parser::<syn::LitByte>(cursor)? },
                    _ => {
                        let impl_name = format_ident!("parse_{}_impl", rule_name_ident.unwrap());
                        quote! { rt::builtins::#impl_name(cursor, ctx)? }
                    }
                };

                let bind_stmt = if let Some(bind) = binding {
                    quote!(let #bind = _val;)
                } else {
                    quote!()
                };
                Ok(quote! {
                    let (_val, next_cursor) = #expr;
                    #bind_stmt
                    let mut cursor = next_cursor;
                })
            } else {
                let func_call = generate_rule_call_expr(rule_path, args, ctx)?;
                let bind_stmt = if let Some(bind) = binding {
                    quote!(let #bind = _val;)
                } else {
                    quote!()
                };
                Ok(quote! {
                    let (_val, next_cursor) = #func_call;
                    #bind_stmt
                    let mut cursor = next_cursor;
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
                        let _start_cursor = cursor;
                        let _res = (|| -> rt::ParseResult<'a, _> {
                            #inner_logic
                            Ok((#return_tuple, cursor))
                        })();
                        match _res {
                            Ok((vals, next_cursor)) => {
                                // Zero-Progress-Schutz (siehe oben).
                                if next_cursor == _start_cursor { break; }
                                let #tuple_pat = vals;
                                #(#push_vecs)*
                                cursor = next_cursor;
                            }
                            Err(e) => {
                                if e.priority >= 50 { return Err(e); }
                                // Die Wiederholung endet regulaer - der Grund wird
                                // gemerkt, sonst geht er hier verloren.
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
                        let _start_cursor = cursor;
                        let _res = (|| -> rt::ParseResult<'a, _> {
                            #inner_logic
                            Ok(((), cursor))
                        })();
                        match _res {
                            Ok((_, next_cursor)) => {
                                // Zero-Progress-Schutz: sonst dreht sich die Schleife ewig,
                                // wenn das innere Muster ohne Tokenverbrauch matcht.
                                if next_cursor == _start_cursor { break; }
                                cursor = next_cursor;
                            }
                            Err(e) => {
                                if e.priority >= 50 { return Err(e); }
                                // Die Wiederholung endet regulaer - der Grund wird
                                // gemerkt, sonst geht er hier verloren.
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
                    // Erstes Pflicht-Element NICHT in einen Block wickeln: #inner_logic
                    // endet mit `let mut cursor = next_cursor;`, das Vorruecken ginge
                    // sonst am Blockende verloren und die Schleife liefe erneut darauf.
                    #inner_logic
                    let #tuple_pat = #return_tuple;
                    #(#push_vecs)*
                    loop {
                        let _start_cursor = cursor;
                        let _res = (|| -> rt::ParseResult<'a, _> {
                            #inner_logic
                            Ok((#return_tuple, cursor))
                        })();
                        match _res {
                            Ok((vals, next_cursor)) => {
                                // Zero-Progress-Schutz (siehe oben).
                                if next_cursor == _start_cursor { break; }
                                let #tuple_pat = vals;
                                #(#push_vecs)*
                                cursor = next_cursor;
                            }
                            Err(e) => {
                                if e.priority >= 50 { return Err(e); }
                                // Die Wiederholung endet regulaer - der Grund wird
                                // gemerkt, sonst geht er hier verloren.
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
                        let _start_cursor = cursor;
                        let _res = (|| -> rt::ParseResult<'a, _> {
                            #inner_logic
                            Ok(((), cursor))
                        })();
                        match _res {
                            Ok((_, next_cursor)) => {
                                // Zero-Progress-Schutz: sonst dreht sich die Schleife ewig,
                                // wenn das innere Muster ohne Tokenverbrauch matcht.
                                if next_cursor == _start_cursor { break; }
                                cursor = next_cursor;
                            }
                            Err(e) => {
                                if e.priority >= 50 { return Err(e); }
                                // Die Wiederholung endet regulaer - der Grund wird
                                // gemerkt, sonst geht er hier verloren.
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
                    let _start_cursor = cursor;
                    let _opt_res = (|| -> rt::ParseResult<'a, _> {
                        #inner_logic
                        Ok(((), cursor))
                    })();
                    match _opt_res {
                        Ok((_, next_cursor)) => { cursor = next_cursor; }
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
                    let _start_cursor = cursor;
                    let _opt_res = (|| -> rt::ParseResult<'a, _> {
                        #inner_logic
                        Ok(( (#(#vars),*) , cursor))
                    })();
                    let (#(#vars),*) = match _opt_res {
                        Ok((vals, next_cursor)) => {
                            cursor = next_cursor;
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
                let (_val, next_cursor) = (|| -> rt::ParseResult<'a, _> { #variant_logic })()?;
                #bind_stmt
                let mut cursor = next_cursor;
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

            // Die Bindings der Gruppe muessen im UMGEBENDEN Scope landen, sonst
            // sieht der Action-Block sie nicht (sie starben frueher mit dem if-let-Block).
            Ok(quote! {
                let (_val, _after_group) = if let Some((inner_cursor, _span, _next_cursor)) = cursor.group(proc_macro2::Delimiter::#delimiter) {
                    // Innerhalb der Gruppe meldet Cursor::eof() das Gruppenende, nicht
                    // das Eingabeende. Die Tiefe merken, damit Meldungen den
                    // Unterschied benennen koennen.
                    ctx.enter_group();
                    let _grp_res = (|| -> rt::ParseResult<'a, _> {
                        let mut cursor = inner_cursor;
                        #inner_logic
                        Ok((#return_expr, cursor))
                    })();
                    ctx.exit_group();
                    let (_val, _inner_end) = _grp_res?;
                    // "unexpected token in delimited group" ist nur ein Platzhalter fuer
                    // "der Inhalt wurde nicht vollstaendig verbraucht". Wurde unterwegs
                    // ein Grund gemerkt, ist der strikt aussagekraeftiger - deshalb
                    // nicht strukturell, damit er nicht gewinnt.
                    if !_inner_end.eof() {
                        return Err(ctx.best_error(
                            rt::ParseError::at_cursor(_inner_end, "unexpected token in delimited group")
                        ));
                    }
                    (_val, _next_cursor)
                } else {
                    return Err(rt::ParseError::at_cursor(cursor, "expected delimited group").with_priority(50));
                };
                #bind_stmt
                let mut cursor = _after_group;
            })
        }
        ModelPattern::LexicalScope(inner, _) => {
            let inner_logic = generate_pattern_step(inner, ctx)?;
            Ok(quote! {
                ctx.enter_lexical();
                // Das `?` MUSS hinter `exit_mode()` stehen - sonst bleibt der
                // Modus bei einem Fehler auf dem Stapel liegen. Der Delimiter-Zweig
                // macht es mit `exit_group()` genauso.
                let _mode_res = (|| -> rt::ParseResult<'a, _> {
                    #inner_logic
                    Ok(((), cursor))
                })();
                ctx.exit_mode();
                let (_res, next_cursor) = _mode_res?;
                let mut cursor = next_cursor;
            })
        }
        ModelPattern::SpacedScope(inner, _) => {
            let inner_logic = generate_pattern_step(inner, ctx)?;
            Ok(quote! {
                ctx.enter_spaced();
                // Das `?` MUSS hinter `exit_mode()` stehen - sonst bleibt der
                // Modus bei einem Fehler auf dem Stapel liegen. Der Delimiter-Zweig
                // macht es mit `exit_group()` genauso.
                let _mode_res = (|| -> rt::ParseResult<'a, _> {
                    #inner_logic
                    Ok(((), cursor))
                })();
                ctx.exit_mode();
                let (_res, next_cursor) = _mode_res?;
                let mut cursor = next_cursor;
            })
        }
        ModelPattern::SpanBinding(inner, span_var, _) => {
            let inner_logic = generate_pattern_step(inner, ctx)?;
            Ok(quote! {
                let _span_start = cursor.span();
                #inner_logic
                let _span_end = cursor.span();
                let #span_var = _span_start.join(_span_end).unwrap_or(_span_start);
            })
        }
        ModelPattern::Recover {
            binding,
            body,
            sync,
            span: _,
        } => {
            // Ein ungebundener Regelaufruf im Body erbt den aeusseren Binding-Namen,
            // sonst haette der Body keinen Wert und recover() lieferte () statt Option<T>.
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

            // Kein Binding: nichts zuzuweisen. Genau eines: _val wird direkt in
            // Some(..) gewickelt, eine Zwischenzuweisung wuerde den Wert vorher
            // wegbewegen. Erst ab zwei wird destrukturiert.
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
                let _start_cursor = cursor;
                let _rec_res = (|| -> rt::ParseResult<'a, _> {
                    #inner_logic
                    Ok((#return_expr, cursor))
                })();
                let (_val, next_cursor) = match _rec_res {
                    Ok((_val, c)) => {
                        #some_assign
                        (#option_ret, c)
                    }
                    Err(e) => {
                        if e.priority >= 50 { return Err(e); }
                        let mut temp_cursor = _start_cursor;
                        loop {
                            if temp_cursor.eof() { break; }
                            if rt::peek_syn(temp_cursor, |i| i.peek(#sync_peek)) { break; }
                            if let Some((_, next)) = temp_cursor.token_tree() { temp_cursor = next; }
                        }
                        (#none_ret, temp_cursor)
                    }
                };
                #bind_stmt
                let mut cursor = next_cursor;
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
                let (_val, _) = (|| -> rt::ParseResult<'a, _> {
                    let mut cursor = cursor; // copy
                    #inner_logic
                    Ok((#return_expr, cursor))
                })()?;
                #bind_stmt
            })
        }
        ModelPattern::Not(inner, _) => {
            let inner_logic = generate_pattern_step(inner, ctx)?;
            // Wenn `not(..)` eine Regel abwehrt, gehoert ihr Name in die Meldung -
            // sonst steht dort nur "unexpected match" ohne jeden Anhaltspunkt.
            let inner_name = match &**inner {
                ModelPattern::RuleCall { rule_path, .. } => rule_path
                    .segments
                    .last()
                    .map(|s| s.ident.to_string().replace('_', " ")),
                _ => None,
            };
            let current_rule = ctx.current_rule.clone();
            let meldung = match inner_name {
                Some(name) => quote! {
                    {
                        let _gefunden = cursor
                            .token_tree()
                            .map(|(tt, _)| tt.to_string())
                            .unwrap_or_default();
                        format!(
                            "unexpected match for rule `{}`; found `{}` in rule `{}`",
                            #name, _gefunden, #current_rule
                        )
                    }
                },
                None => quote!("unexpected match".to_string()),
            };
            Ok(quote! {
                let _not_res = (|| -> rt::ParseResult<'a, _> {
                    let mut cursor = cursor; // copy
                    #inner_logic
                    Ok(((), cursor))
                })();
                if _not_res.is_ok() {
                    return Err(rt::ParseError::at_cursor(cursor, #meldung).with_priority(50));
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
                    if cursor.eof() { break; }
                    let _is_match = (|| -> rt::ParseResult<'a, _> {
                        let mut cursor = cursor; // copy
                        #inner_logic
                        Ok(((), cursor))
                    })().is_ok();
                    if _is_match { break; }
                    if let Some((tt, next)) = cursor.token_tree() {
                        _tokens.push(tt);
                        cursor = next;
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
            // count(..) zaehlt das ELEMENT, nicht den Wiederholungs-Operator.
            // `count("a"*)` auf "a a a" ist 3, nicht 1: also muss der Operator
            // abgestreift und sein Element gezaehlt werden. Eine generische
            // Schleife ueber "a"* wuerde beim ersten Durchlauf alles verbrauchen
            // und danach endlos leer weiterlaufen.
            let bind_stmt = if let Some(bind) = binding {
                quote!(let #bind = _count;)
            } else {
                quote!(let _ = _count;)
            };

            // Ein Schleifendurchlauf ueber das Element, mit Zero-Progress-Schutz.
            let loop_over = |elem_logic: &proc_macro2::TokenStream| {
                quote! {
                    loop {
                        let _start_cursor = cursor;
                        let _res = (|| -> rt::ParseResult<'a, _> {
                            #elem_logic
                            Ok(((), cursor))
                        })();
                        match _res {
                            Ok((_, next_cursor)) => {
                                if next_cursor == _start_cursor { break; }
                                cursor = next_cursor;
                                _count += 1;
                            }
                            Err(e) => {
                                if e.priority >= 50 { return Err(e); }
                                // Die Wiederholung endet regulaer - der Grund wird
                                // gemerkt, sonst geht er hier verloren.
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
                    // Erstes Element ist Pflicht: nicht in einen Block wickeln,
                    // sonst geht das Vorruecken des Cursors beim Blockende verloren.
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
                        let _start_cursor = cursor;
                        let _res = (|| -> rt::ParseResult<'a, _> {
                            #elem_logic
                            Ok(((), cursor))
                        })();
                        match _res {
                            Ok((_, next_cursor)) => { cursor = next_cursor; _count += 1; }
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
                quote! { return Err(rt::ParseError::at_cursor(cursor, #arg_expr).with_priority(50)); },
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
                quote!(#rule_path(cursor)?)
            } else {
                quote!(#rule_path(cursor, #(#arg_exprs),*)?)
            })
        } else if ctx.grammar.rules.iter().any(|r| r.name == *ident) {
            let impl_name = format_ident!("parse_{}_impl", ident);
            Ok(if arg_exprs.is_empty() {
                quote!(#impl_name(cursor, ctx)?)
            } else {
                quote!(#impl_name(cursor, ctx, #(#arg_exprs),*)?)
            })
        } else {
            Ok(if arg_exprs.is_empty() {
                quote!(#rule_path(cursor)?)
            } else {
                quote!(#rule_path(cursor, #(#arg_exprs),*)?)
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
                quote!(#new_path(cursor, ctx)?)
            } else {
                quote!(#new_path(cursor, ctx, #(#arg_exprs),*)?)
            })
        } else {
            Ok(if arg_exprs.is_empty() {
                quote!(#rule_path(cursor)?)
            } else {
                quote!(#rule_path(cursor, #(#arg_exprs),*)?)
            })
        }
    }
}
