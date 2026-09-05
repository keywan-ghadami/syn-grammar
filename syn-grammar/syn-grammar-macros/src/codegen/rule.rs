use super::pattern;
use super::CodegenContext;
use crate::backend::SynBackend;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use std::collections::HashMap;
use syn::Result;
use syn_grammar_model::{analysis, model::*, Backend};

pub fn generate_rule(rule: &Rule, ctx: &CodegenContext) -> Result<TokenStream> {
    let name = &rule.name;
    let context_name = name.to_string().replace("_", " ");
    let fn_name = format_ident!("parse_{}", name);
    let impl_name = format_ident!("parse_{}_impl", name);
    let ret_type = &rule.return_type;
    let attrs = &rule.attrs;
    let generics = &rule.generics;

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
    let (recursive_refs, base_refs) = analysis::split_left_recursive(name, &rule.variants);
    let where_clause = &generics.where_clause;

    // Separate context for this rule so that deeper patterns know its name.
    let rule_ctx = CodegenContext {
        grammar: ctx.grammar,
        custom_keywords: ctx.custom_keywords,
        current_rule: context_name.clone(),
    };
    let ctx = &rule_ctx;

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
                let base_parser = |input: &rt::Stream<'a>, ctx: &mut rt::ParseContext<'a>| -> rt::StreamResult<'a, #ret_type> {
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

    Ok(quote! {
        #(#attrs)*
        #default_doc
        #vis fn #fn_name(input: syn::parse::ParseStream #(#params)*) -> syn::Result<#ret_type> #where_clause {
            // `ParseStream<'z>` is `&'z ParseBuffer<'z>` and thus fits directly
            // onto `&rt::Stream<'a>` with `'a = 'z`. The stream is advanced by the rule
            // itself - the `input.step` detour that used to be needed (only there was
            // the cursor lifetime nameable) is gone.
            let mut ctx = rt::ParseContext::new();
            match #impl_name(input, &mut ctx #(#param_names)*) {
                Ok(res) => {
                    // The rule succeeded but did not consume everything.
                    // If a reason why it could not continue was recorded along the way,
                    // that is the answer - otherwise syn only reports "unexpected token".
                    if !input.is_empty() {
                        if let Some(f) = ctx.furthest.clone() {
                            let mut f = f;
                            f.push_rule(#context_name);
                            return Err(syn::Error::new(f.span, f.to_string()));
                        }
                    }
                    Ok(res)
                }
                Err(e) => {
                    // The returned error is not necessarily the most
                    // informative one - one that got further may have been covered up
                    // along the way by a successful backtrack.
                    let mut e = ctx.best_error(e);
                    e.push_rule(#context_name);
                    Err(syn::Error::new(e.span, e.to_string()))
                }
            }
        }

        #[doc(hidden)]
        #(#impl_attrs)*
        pub fn #impl_name<'a>(input: &rt::Stream<'a>, ctx: &mut rt::ParseContext<'a> #(#params)*) -> rt::StreamResult<'a, #ret_type> #where_clause {
            // The rule name sits on the live stack for the duration of the body, so that
            // an error recorded here (rather than passed out) picks it up.
            // enter/exit enclose the body closure - all early-returning
            // paths inside it (cut, is_fatal, unique peek) only leave the closure, so the
            // pair automatically stays balanced.
            ctx.enter_rule(#context_name);
            #lexical_block_start
            let _res = (|| -> rt::StreamResult<'a, #ret_type> {
                #body
            })();
            #lexical_block_end
            ctx.exit_rule();

            match _res {
                Ok(res) => Ok(res),
                Err(mut e) => {
                    e.push_rule(#context_name);
                    Err(e)
                }
            }
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

        let bind_stmt = if let Some(b) = lhs_binding { quote! { let #b = lhs.clone(); } } else { quote! {} };
        let logic = pattern::generate_sequence(tail_pattern, &variant.action, ctx)?;
        let peek_token_obj = tail_pattern.first().and_then(|f| analysis::get_simple_peek(f, ctx.custom_keywords).ok().flatten());

        let arm_logic = quote! {
            let _start_cursor = input.cursor();
            // Try on a fork: if the arm fails, the stream stays
            // where it was. With the cursor design this was free (simply don't use
            // the new cursor), here it costs the fork.
            let _fork = rt::fork(input);
            let _arm_res = (|| -> rt::StreamResult<'a, _> {
                let input = &_fork;
                #bind_stmt
                #logic
            })();

            match _arm_res {
                Ok(new_lhs) => {
                    let next_cursor = _fork.cursor();
                    // Cursor comparison, not position comparison: `Cursor` is
                    // `PartialEq` (pointer equality within the shared TokenBuffer),
                    // and that is exactly the question - did the parser stand at
                    // the same place afterwards? Via `span().start()`, ALL positions
                    // would have been (0,0) in a procedural macro up to Rust 1.87, whereby
                    // every left-recursive rule would have aborted immediately.
                    if _start_cursor == next_cursor {
                        return Err(rt::ParseError::at_cursor(_start_cursor, "Left-recursive rule matched empty string").with_priority(50));
                    }
                    rt::advance_to(input, &_fork);
                    lhs = new_lhs;
                    continue;
                }
                Err(e) => {
                    if e.priority >= 50 { return Err(e); }
                    // The loop ends here regularly with the `lhs` obtained so far.
                    // Without recording this, the reason why no further expansion
                    // happened would be lost without replacement.
                    ctx.record_failure(&e);
                }
            }
        };

        if let Some(token_code) = peek_token_obj {
            Ok(quote! { if rt::peek_syn(input.cursor(), #token_code) { #arm_logic } })
        } else {
            Ok(arm_logic)
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
        return Ok(
            quote! { Err(rt::ParseError::at_cursor(input.cursor(), "No variants defined")) },
        );
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

    let arms = variants.iter().map(|variant| {
        // An alternative that is nothing but a call to another rule can name
        // that rule. Precedence: the label at the call site, then the label the
        // called rule gives itself, then the grouped form `rule(`a`, `b`)`
        // built at runtime from what the rule turned out to accept.
        let called_rule = single_rule_call(&variant.pattern, ctx);
        let inherited_label = called_rule
            .as_ref()
            .and_then(|r| ctx.grammar.rules.iter().find(|d| d.name == *r))
            .and_then(|d| d.label.clone());
        let label_str = if let Some(l) = &variant.label { Some(l.clone()) }
            else if let Some(l) = inherited_label { Some(l) }
            else { analysis::expectation_label(&variant.pattern) };
        let label_lit = if let Some(l) = &label_str { quote!(Some(#l)) } else { quote!(None::<&str>) };
        let group_lit = match (&label_str, &called_rule) {
            (None, Some(r)) => {
                let display = r.to_string().replace('_', " ");
                quote!(Some(#display))
            }
            _ => quote!(None::<&str>),
        };

        let cut_info = analysis::find_cut(&variant.pattern);
        let first_pat = variant.pattern.first();
        let is_nullable = first_pat.is_none_or(analysis::is_nullable);
        let peek_token_obj = if !is_nullable { first_pat.and_then(|f| analysis::get_simple_peek(f, ctx.custom_keywords).ok().flatten()) } else { None };
        let peek_str = if !is_nullable { analysis::get_peek_token_string(&variant.pattern) } else { None };
        let is_unique = if let (_, Some(token_key)) = (&peek_token_obj, &peek_str) {
            token_counts.get(token_key).map(|c| *c == 1).unwrap_or(false)
        } else { false };

        let logic = if let Some(cut) = cut_info {
            let pre_logic = pattern::generate_sequence_steps(cut.pre_cut, ctx)?;
            let post_logic = pattern::generate_sequence_steps(cut.post_cut, ctx)?;
            let action = &variant.action;
            let pre_bindings = analysis::collect_bindings(cut.pre_cut);

            let cut_block = quote! {
                let _fork = rt::fork(input);
                let pre_res = (|| -> rt::StreamResult<'a, _> {
                    let input = &_fork;
                    #pre_logic
                    Ok((#(#pre_bindings),*))
                })();

                match pre_res {
                    Ok((#(#pre_bindings),*)) => {
                        // The part before the cut succeeded. From here on everything
                        // continues on the same fork; an error after this is
                        // fatal, so there is no backtracking anyway.
                        let post_res = (|| -> rt::StreamResult<'a, _> {
                            let input = &_fork;
                            #post_logic
                            Ok((|| -> syn::Result<_> { Ok({ #action }) })().map_err(rt::ParseError::from)?)
                        })();
                        match post_res {
                            Ok(res) => {
                                rt::advance_to(input, &_fork);
                                return Ok(res);
                            }
                            // CUT: the derivation is fixed, backtracking is pointless.
                            Err(e) => return Err(e.as_fatal()),
                        }
                    }
                    Err(e) => {
                        // Only a cut short-circuits. A `fail(..)` has high priority,
                        // but is not fatal - it must take part in the comparison, otherwise
                        // it also wins against an error that got further.
                        if e.is_fatal { return Err(e); }
                        // Without progress, the alternative failed at its boundary.
                        // Then its label counts as an expectation, instead of carrying
                        // the internal message outward (ADR 13, item 6).
                        if e.at == Some(_start_cursor) {
                            if let Some(lbl) = #label_lit { _expected.push(lbl.to_string()); }
                            // A plain rule call names the rule and what it
                            // accepts: `term(`integer literal`, `parentheses`)`.
                            else if let Some(rule) = #group_lit {
                                rt::push_grouped(&mut _expected, rule, &e.expected);
                            }
                            // Otherwise what the branch itself would have accepted -
                            // a built-in's expectation, or the union an inner rule
                            // already collected. Without this, only peekable
                            // branches made it into `expected one of:`.
                            else { _expected.extend(e.expected.iter().cloned()); }
                        }
                        // Only WITH progress into the high-water mark: if a later
                        // alternative wins, `_best_err` is discarded - and with
                        // it the most informative reason, in case input is still
                        // left over afterwards. An error without progress, on the other
                        // hand, is not a "furthest failure point"; it belongs in the
                        // expectation list above, not in the global mark - otherwise
                        // the aggregation of an optional sub-pattern displaces the
                        // label of the item (ADR 13, item 6).
                        else {
                            ctx.record_failure(&e);
                        }
                        _best_err = Some(_best_err.map_or(e.clone(), |b| b.merge(e)));
                    }
                }
            };
            cut_block
        } else {
            let inner_logic = pattern::generate_sequence(&variant.pattern, &variant.action, ctx)?;
            quote! {
                let _fork = rt::fork(input);
                let _arm_res = (|| -> rt::StreamResult<'a, _> {
                    let input = &_fork;
                    #inner_logic
                })();
                match _arm_res {
                    Ok(res) => {
                        rt::advance_to(input, &_fork);
                        return Ok(res);
                    }
                    Err(e) => {
                        // Only a cut short-circuits. A `fail(..)` has high priority,
                        // but is not fatal - it must take part in the comparison, otherwise
                        // it also wins against an error that got further.
                        if e.is_fatal { return Err(e); }
                        // Without progress, the alternative failed at its boundary.
                        // Then its label counts as an expectation, instead of carrying
                        // the internal message outward (ADR 13, item 6).
                        if e.at == Some(_start_cursor) {
                            if let Some(lbl) = #label_lit { _expected.push(lbl.to_string()); }
                            // A plain rule call names the rule and what it
                            // accepts: `term(`integer literal`, `parentheses`)`.
                            else if let Some(rule) = #group_lit {
                                rt::push_grouped(&mut _expected, rule, &e.expected);
                            }
                            // Otherwise what the branch itself would have accepted -
                            // a built-in's expectation, or the union an inner rule
                            // already collected. Without this, only peekable
                            // branches made it into `expected one of:`.
                            else { _expected.extend(e.expected.iter().cloned()); }
                        }
                        // Only WITH progress into the high-water mark: if a later
                        // alternative wins, `_best_err` is discarded - and with
                        // it the most informative reason, in case input is still
                        // left over afterwards. An error without progress, on the other
                        // hand, is not a "furthest failure point"; it belongs in the
                        // expectation list above, not in the global mark - otherwise
                        // the aggregation of an optional sub-pattern displaces the
                        // label of the item (ADR 13, item 6).
                        else {
                            ctx.record_failure(&e);
                        }
                        _best_err = Some(_best_err.map_or(e.clone(), |b| b.merge(e)));
                    }
                }
            }
        };

        // If the peek fails, the branch does not run at all and produces no
        // error. Its label is nevertheless a valid expectation at this
        // position and must go into the enumeration (ADR 13, item 6) - otherwise only
        // the meaningless message "No matching rule variant found" remains.
        let otherwise_expected = quote! {
            else if let Some(lbl) = #label_lit { _expected.push(lbl.to_string()); }
        };

        if is_unique {
            let token_code = peek_token_obj.as_ref().unwrap();
            Ok(quote! {
                if rt::peek_syn(_start_cursor, #token_code) {
                    #logic
                    // The peek token belongs unambiguously to this variant: if it
                    // fails, the remaining ones need not be tried at all.
                    // The `return` already does that - the priority is NOT
                    // raised. A structural error would be fatal here in the sense
                    // of "not recoverable", and exactly that made recover() unusable on every
                    // rule with a unique start token.
                    if let Some(err) = _best_err.take() {
                        return Err(err);
                    } else {
                        return Err(rt::ParseError::at_cursor(_start_cursor, "propagating unique variant error"));
                    }
                }
                #otherwise_expected
            })
        } else if let Some(token_code) = peek_token_obj {
            Ok(quote! {
                if rt::peek_syn(_start_cursor, #token_code) { #logic }
                #otherwise_expected
            })
        } else {
            Ok(logic)
        }
    }).collect::<Result<Vec<_>>>()?;

    let error_msg = if is_top_level {
        "No matching rule variant found"
    } else {
        "No matching variant in group"
    };

    Ok(quote! {
        let mut _best_err: Option<rt::ParseError> = None;
        let mut _expected: Vec<String> = Vec::new();
        let _start_cursor = input.cursor();

        #(#arms)*

        Err(rt::finish_variants(_best_err, _expected, _start_cursor, #error_msg, ctx.end_of_scope_msg()))
    })
}

/// The rule an alternative consists of, if it consists of nothing else.
///
/// Only then can the alternative be named after it: `s:shared_struct` is a
/// shared struct, whereas `attrs:outer_attrs "struct" …` merely starts with a
/// rule call and is not that rule. Built-ins and `syn::` types name themselves
/// already (`identifier`, `Rust type`) and are left alone.
fn single_rule_call(pattern: &[ModelPattern], ctx: &CodegenContext) -> Option<Ident> {
    let [ModelPattern::RuleCall {
        rule_path, args, ..
    }] = pattern
    else {
        return None;
    };
    if !args.is_empty() {
        return None;
    }
    let name = rule_path.get_ident()?;
    let is_builtin = SynBackend::get_builtins().iter().any(|b| name == b.name);
    if is_builtin || !ctx.grammar.rules.iter().any(|r| r.name == *name) {
        return None;
    }
    Some(name.clone())
}
