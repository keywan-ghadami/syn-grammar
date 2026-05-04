use super::pattern;
use super::CodegenContext;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::collections::HashMap;
use syn::Result;
use syn_grammar_model::{analysis, model::*};

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
            p.is_ident("cfg") || p.is_ident("cfg_attr") || p.is_ident("allow") || p.is_ident("warn") || p.is_ident("deny") || p.is_ident("forbid")
        })
        .collect();

    let default_doc = if attrs.iter().any(|a| a.path().is_ident("doc")) {
        quote!()
    } else {
        let msg = format!("Parser for rule `{}`.", name);
        quote!(#[doc = #msg])
    };

    let params: Vec<_> = rule.params.iter().filter_map(|p| p.ty.as_ref().map(|t| { let name = &p.name; quote! { , #name : #t } })).collect();
    let param_names: Vec<_> = rule.params.iter().filter_map(|p| p.ty.as_ref().map(|_| { let name = &p.name; quote! { , #name } })).collect();

    let is_public = rule.is_pub || name == "main";
    let vis = if is_public { quote!(pub) } else { quote!() };
    let (recursive_refs, base_refs) = analysis::split_left_recursive(name, &rule.variants);
    let where_clause = &generics.where_clause;

    let lexical_block_start = if rule.is_lexical { quote! { ctx.enter_lexical(); } } else { quote! {} };
    let lexical_block_end = if rule.is_lexical { quote! { ctx.exit_mode(); } } else { quote! {} };

    let body = if !recursive_refs.is_empty() {
        if base_refs.is_empty() {
            return Err(syn::Error::new(name.span(), "Left-recursive rule requires at least one non-recursive base variant."));
        }

        let base_owned: Vec<RuleVariant> = base_refs.into_iter().cloned().collect();
        let recursive_owned: Vec<RuleVariant> = recursive_refs.into_iter().cloned().collect();
        let base_logic = generate_variants_internal(&base_owned, true, ctx)?;
        let loop_logic = generate_recursive_loop_body(&recursive_owned, ctx)?;

        quote! {
            let mut lhs = {
                let base_parser = |cursor: syn::buffer::Cursor<'a>, ctx: &mut rt::ParseContext| -> rt::ParseResult<'a, #ret_type> {
                    #base_logic
                };
                let (val, next_cursor) = base_parser(cursor, ctx)?;
                cursor = next_cursor;
                val
            };
            loop {
                #loop_logic
                break;
            }
            Ok((lhs, cursor))
        }
    } else {
        generate_variants_internal(&rule.variants, true, ctx)?
    };

    Ok(quote! {
        #(#attrs)*
        #default_doc
        // PUBLIC API: Akzeptiert ParseStream (Zustand), nutzt Cursor intern und synchronisiert!
        #vis fn #fn_name(input: syn::parse::ParseStream #(#params)*) -> syn::Result<#ret_type> #where_clause {
            let cursor = input.cursor();
            let mut ctx = rt::ParseContext::new();
            match #impl_name(cursor, &mut ctx #(#param_names)*) {
                Ok((res, next_cursor)) => {
                    // Syn's ParseStream auf den fortgeschrittenen Cursor setzen!
                    input.step(|_| Ok(((), next_cursor))).unwrap();
                    Ok(res)
                }
                Err(mut e) => {
                    e.push_rule(#context_name);
                    Err(syn::Error::new(e.span, e.to_string()))
                }
            }
        }

        #[doc(hidden)]
        #(#impl_attrs)*
        pub fn #impl_name<'a>(mut cursor: syn::buffer::Cursor<'a>, ctx: &mut rt::ParseContext #(#params)*) -> rt::ParseResult<'a, #ret_type> #where_clause {
            #lexical_block_start
            let _res = (|| -> rt::ParseResult<'a, #ret_type> {
                #body
            })();
            #lexical_block_end
            
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

fn generate_recursive_loop_body(variants: &[RuleVariant], ctx: &CodegenContext) -> Result<TokenStream> {
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
            let _start_cursor = cursor;
            let _arm_res = (|| -> rt::ParseResult<_> {
                let mut cursor = _start_cursor;
                #bind_stmt
                #logic
            })();

            match _arm_res {
                Ok((new_lhs, next_cursor)) => {
                    if _start_cursor.span().start() == next_cursor.span().start() {
                        return Err(rt::ParseError::new(_start_cursor.span(), "Left-recursive rule matched empty string").with_priority(50));
                    }
                    lhs = new_lhs;
                    cursor = next_cursor;
                    continue;
                }
                Err(e) => {
                    if e.priority >= 50 { return Err(e); }
                }
            }
        };

        if let Some(token_code) = peek_token_obj {
            Ok(quote! { if rt::peek_syn(cursor, |i| i.peek(#token_code)) { #arm_logic } })
        } else {
            Ok(arm_logic)
        }
    }).collect::<Result<Vec<_>>>()?;
    Ok(quote! { #(#arms)* })
}

pub fn generate_variants_internal(variants: &[RuleVariant], is_top_level: bool, ctx: &CodegenContext) -> Result<TokenStream> {
    if variants.is_empty() { return Ok(quote! { Err(rt::ParseError::new(cursor.span(), "No variants defined")) }); }

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
        let label_str = if let Some(l) = &variant.label { Some(l.clone()) } else { analysis::get_peek_token_string(&variant.pattern) };
        let label_lit = if let Some(l) = &label_str { quote!(Some(#l)) } else { quote!(None) };

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
                let pre_res = (|| -> rt::ParseResult<_> {
                    let mut cursor = _start_cursor;
                    #pre_logic
                    Ok(((#(#pre_bindings),*), cursor))
                })();

                match pre_res {
                    Ok(((#(#pre_bindings),*), mut cursor)) => {
                        let post_res = (|| -> rt::ParseResult<_> {
                            #post_logic
                            Ok(( { #action }, cursor ))
                        })();
                        match post_res {
                            Ok(res) => return Ok(res),
                            Err(e) => return Err(e.with_priority(50)), // CUT: fatal!
                        }
                    }
                    Err(mut e) => {
                        if let Some(lbl) = #label_lit {
                            if e.span.start() == _start_cursor.span().start() {
                                e.message = format!("expected {}", lbl);
                                e.priority = std::cmp::max(e.priority, 10);
                            }
                        }
                        if e.priority >= 50 { return Err(e); }
                        _best_err = Some(_best_err.map_or(e.clone(), |b| b.merge(e)));
                    }
                }
            };
            cut_block
        } else {
            let inner_logic = pattern::generate_sequence(&variant.pattern, &variant.action, ctx)?;
            quote! {
                let _arm_res = (|| -> rt::ParseResult<_> {
                    let mut cursor = _start_cursor;
                    #inner_logic
                })();
                match _arm_res {
                    Ok(res) => return Ok(res),
                    Err(mut e) => {
                        if let Some(lbl) = #label_lit {
                            if e.span.start() == _start_cursor.span().start() {
                                e.message = format!("expected {}", lbl);
                                e.priority = std::cmp::max(e.priority, 10);
                            }
                        }
                        if e.priority >= 50 { return Err(e); }
                        _best_err = Some(_best_err.map_or(e.clone(), |b| b.merge(e)));
                    }
                }
            }
        };

        if is_unique {
            let token_code = peek_token_obj.as_ref().unwrap();
            Ok(quote! {
                if rt::peek_syn(_start_cursor, |i| i.peek(#token_code)) {
                    #logic
                    // Wenn wir hierher kommen, ist der Zweig fatal gescheitert!
                    if let Some(err) = _best_err.take() {
                        return Err(err.with_priority(50));
                    } else {
                        return Err(rt::ParseError::new(_start_cursor.span(), "propagating fatal unique error").with_priority(50));
                    }
                }
            })
        } else if let Some(token_code) = peek_token_obj {
            Ok(quote! { if rt::peek_syn(_start_cursor, |i| i.peek(#token_code)) { #logic } })
        } else {
            Ok(logic)
        }
    }).collect::<Result<Vec<_>>>()?;

    let error_msg = if is_top_level { "No matching rule variant found" } else { "No matching variant in group" };

    Ok(quote! {
        let mut _best_err: Option<rt::ParseError> = None;
        let _start_cursor = cursor;

        #(#arms)*

        Err(_best_err.unwrap_or_else(|| rt::ParseError::new(_start_cursor.span(), #error_msg)))
    })
}
