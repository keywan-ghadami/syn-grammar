#![feature(prelude_import)]
#[macro_use]
extern crate std;
#[prelude_import]
use std::prelude::rust_2021::*;
use syn::{self, Attribute, Ident, LitStr, Macro};
use syn::parse::{Parse, ParseStream, Result};
use syn_grammar::grammar;
pub struct FfiMod {
    pub attrs: Vec<Attribute>,
    pub name: Ident,
    pub blocks: Vec<ExternBlock>,
}
#[automatically_derived]
impl ::core::fmt::Debug for FfiMod {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::debug_struct_field3_finish(
            f,
            "FfiMod",
            "attrs",
            &self.attrs,
            "name",
            &self.name,
            "blocks",
            &&self.blocks,
        )
    }
}
pub struct ExternBlock {
    pub is_unsafe: bool,
    pub lang: LitStr,
    pub items: Vec<CxxItem>,
}
#[automatically_derived]
impl ::core::fmt::Debug for ExternBlock {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::debug_struct_field3_finish(
            f,
            "ExternBlock",
            "is_unsafe",
            &self.is_unsafe,
            "lang",
            &self.lang,
            "items",
            &&self.items,
        )
    }
}
pub enum CxxItem {
    Type(Vec<Attribute>, Ident, syn::Generics),
    Function(Vec<Attribute>, Ident, syn::Generics, Vec<CxxArg>, syn::ReturnType),
    Macro(Macro),
}
#[automatically_derived]
impl ::core::fmt::Debug for CxxItem {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match self {
            CxxItem::Type(__self_0, __self_1, __self_2) => {
                ::core::fmt::Formatter::debug_tuple_field3_finish(
                    f,
                    "Type",
                    __self_0,
                    __self_1,
                    &__self_2,
                )
            }
            CxxItem::Function(__self_0, __self_1, __self_2, __self_3, __self_4) => {
                ::core::fmt::Formatter::debug_tuple_field5_finish(
                    f,
                    "Function",
                    __self_0,
                    __self_1,
                    __self_2,
                    __self_3,
                    &__self_4,
                )
            }
            CxxItem::Macro(__self_0) => {
                ::core::fmt::Formatter::debug_tuple_field1_finish(f, "Macro", &__self_0)
            }
        }
    }
}
pub struct CxxArg {
    pub name: Ident,
    pub ty: syn::Type,
}
#[automatically_derived]
impl ::core::fmt::Debug for CxxArg {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::debug_struct_field2_finish(
            f,
            "CxxArg",
            "name",
            &self.name,
            "ty",
            &&self.ty,
        )
    }
}
#[allow(non_snake_case)]
pub mod CxxParser {
    #![allow(unused_imports, unused_variables, dead_code, unused_braces, unused_parens)]
    #![allow(clippy::all)]
    pub const GRAMMAR_NAME: &str = "CxxParser";
    /// The generated source code of the rules, used for testing verification.
    pub const GENERATED_SOURCE: &str = "#[doc = \"Parser for rule `cxx_arg`.\"] fn parse_cxx_arg(input : ParseStream) ->\nResult < CxxArg >\n{\n    let mut ctx = rt :: ParseContext :: new(); match\n    parse_cxx_arg_impl(input, & mut ctx)\n    {\n        Ok(val) => Ok(val), Err(e) =>\n        {\n            if let Some(best) = ctx.take_best_error() { Err(best) } else\n            { Err(e) }\n        }\n    }\n} #[doc(hidden)] pub fn\nparse_cxx_arg_impl(mut input : ParseStream, ctx : & mut rt :: ParseContext) ->\nResult < CxxArg >\n{\n    ctx.enter_rule(stringify! (cxx_arg)); let _start_span = input.span(); let\n    res =\n    (|| -> syn :: Result < CxxArg >\n    {\n        let mut _shallow_failures = Vec :: < & str > :: new(); let _start_span\n        = input.span();\n        {\n            if let Some(res) = rt ::\n            attempt_labeled(input, ctx, None, | mut input, ctx |\n            {\n                {\n                    let name = syn_grammar :: builtins ::\n                    parse_any_ident_impl(& mut input, ctx) ? ; let _t =\n                    input.parse :: < Token! [:] > () ? ;\n                    ctx.record_span(syn :: spanned :: Spanned :: span(& _t)) ? ;\n                    let ty =\n                    {\n                        let start_span = input.span(); eprintln!\n                        (\"[TRACE] Attempting to parse syn type: {} at {:?}\",\n                        stringify! (syn :: Type), start_span); match input.parse ::\n                        < syn :: Type > ()\n                        {\n                            Ok(val) =>\n                            {\n                                eprintln!\n                                (\"[TRACE] Successfully parsed syn type: {}\", stringify!\n                                (syn :: Type)); Ok(val)\n                            }, Err(e) =>\n                            {\n                                eprintln!\n                                (\"[TRACE] FAILED to parse syn type: {}. Error: '{}', Error Span: {:?}\",\n                                stringify! (syn :: Type), e.to_string(), e.span());\n                                eprintln! (\"[TRACE] Start span was: {:?}\", start_span); if\n                                e.span().start() > start_span.start()\n                                {\n                                    eprintln!\n                                    (\"[TRACE] Progress was made. Calling trigger_fail()\");\n                                    ctx.trigger_fail();\n                                } else\n                                {\n                                    eprintln!\n                                    (\"[TRACE] No progress. NOT calling trigger_fail()\");\n                                } Err(e)\n                            }\n                        }\n                    } ? ; Ok({ CxxArg { name: name.into(), ty } })\n                }\n            }) ? { return Ok(res); }\n        } if ctx.stop_aggregation(_start_span)\n        {\n            if let Some(best_err) = ctx.take_best_error()\n            { return Err(best_err); }\n        } let mut error_to_return = if ! _shallow_failures.is_empty()\n        {\n            _shallow_failures.sort(); _shallow_failures.dedup(); let msg = if\n            _shallow_failures.len() == 1\n            { format! (\"expected `{}`\", _shallow_failures [0]) } else\n            {\n                let joined =\n                _shallow_failures.iter().map(| s | format!\n                (\"`{}`\", s)).collect :: < Vec < _ >> ().join(\", \"); format!\n                (\"expected one of: {}\", joined)\n            }; let _ = ctx.take_best_error(); input.error(msg)\n        } else if let Some(best_err) = ctx.take_best_error() { best_err } else\n        { input.error(\"No matching rule variant found\") }; if !\n        input.is_empty()\n        {\n            if let Ok(tt) = input.fork().parse :: < proc_macro2 :: TokenTree >\n            ()\n            {\n                let found = tt.to_string(); if ! found.trim().is_empty()\n                {\n                    let new_message = format!\n                    (\"{}; found unexpected token `{}`\", error_to_return, found);\n                    error_to_return = syn :: Error ::\n                    new(error_to_return.span(), new_message);\n                }\n            }\n        } Err(error_to_return)\n    }) (); if let Err(ref e) = res\n    { ctx.record_error(e.clone(), _start_span, None, 0); } ctx.exit_rule();\n    match res { Ok(val) => Ok(val), Err(e) => { Err(e) } }\n} #[doc = \"Parser for rule `cxx_arg_list`.\"] fn\nparse_cxx_arg_list(input : ParseStream) -> Result < Vec < CxxArg > >\n{\n    let mut ctx = rt :: ParseContext :: new(); match\n    parse_cxx_arg_list_impl(input, & mut ctx)\n    {\n        Ok(val) => Ok(val), Err(e) =>\n        {\n            if let Some(best) = ctx.take_best_error() { Err(best) } else\n            { Err(e) }\n        }\n    }\n} #[doc(hidden)] pub fn\nparse_cxx_arg_list_impl(mut input : ParseStream, ctx : & mut rt ::\nParseContext) -> Result < Vec < CxxArg > >\n{\n    ctx.enter_rule(stringify! (cxx_arg_list)); let _start_span = input.span();\n    let res =\n    (|| -> syn :: Result < Vec < CxxArg > >\n    {\n        let mut _shallow_failures = Vec :: < & str > :: new(); let _start_span\n        = input.span();\n        {\n            if let Some(res) = rt ::\n            attempt_labeled(input, ctx, Some(\"separated\"), | mut input, ctx |\n            {\n                {\n                    let items =\n                    {\n                        let _items_vec = rt :: parse_separated :: < _, _, _ >\n                        (input, ctx, | mut input, ctx |\n                        {\n                            let _item = parse_cxx_arg_impl(& mut input, ctx) ? ;\n                            Ok(_item)\n                        }, | mut input, ctx |\n                        {\n                            let _t = input.parse :: < Token! [,] > () ? ;\n                            ctx.record_span(syn :: spanned :: Spanned :: span(& _t)) ? ;\n                            Ok(())\n                        }, 0usize, false, None) ? ; let mut _items = Vec ::\n                        from_iter(_items_vec); _items\n                    }; Ok({ items })\n                }\n            }) ? { return Ok(res); }\n        } if ! ctx.stop_aggregation(input.span())\n        { _shallow_failures.push(\"separated\"); } if\n        ctx.stop_aggregation(_start_span)\n        {\n            if let Some(best_err) = ctx.take_best_error()\n            { return Err(best_err); }\n        } let mut error_to_return = if ! _shallow_failures.is_empty()\n        {\n            _shallow_failures.sort(); _shallow_failures.dedup(); let msg = if\n            _shallow_failures.len() == 1\n            { format! (\"expected `{}`\", _shallow_failures [0]) } else\n            {\n                let joined =\n                _shallow_failures.iter().map(| s | format!\n                (\"`{}`\", s)).collect :: < Vec < _ >> ().join(\", \"); format!\n                (\"expected one of: {}\", joined)\n            }; let _ = ctx.take_best_error(); input.error(msg)\n        } else if let Some(best_err) = ctx.take_best_error() { best_err } else\n        { input.error(\"No matching rule variant found\") }; if !\n        input.is_empty()\n        {\n            if let Ok(tt) = input.fork().parse :: < proc_macro2 :: TokenTree >\n            ()\n            {\n                let found = tt.to_string(); if ! found.trim().is_empty()\n                {\n                    let new_message = format!\n                    (\"{}; found unexpected token `{}`\", error_to_return, found);\n                    error_to_return = syn :: Error ::\n                    new(error_to_return.span(), new_message);\n                }\n            }\n        } Err(error_to_return)\n    }) (); if let Err(ref e) = res\n    { ctx.record_error(e.clone(), _start_span, None, 0); } ctx.exit_rule();\n    match res { Ok(val) => Ok(val), Err(e) => { Err(e) } }\n} #[doc = \"Parser for rule `cxx_item`.\"] fn\nparse_cxx_item(input : ParseStream) -> Result < CxxItem >\n{\n    let mut ctx = rt :: ParseContext :: new(); match\n    parse_cxx_item_impl(input, & mut ctx)\n    {\n        Ok(val) => Ok(val), Err(e) =>\n        {\n            if let Some(best) = ctx.take_best_error() { Err(best) } else\n            { Err(e) }\n        }\n    }\n} #[doc(hidden)] pub fn\nparse_cxx_item_impl(mut input : ParseStream, ctx : & mut rt :: ParseContext)\n-> Result < CxxItem >\n{\n    ctx.enter_rule(stringify! (cxx_item)); let _start_span = input.span(); let\n    res =\n    (|| -> syn :: Result < CxxItem >\n    {\n        let mut _shallow_failures = Vec :: < & str > :: new(); let _start_span\n        = input.span();\n        {\n            if let Some(res) = rt ::\n            attempt_labeled(input, ctx, None, | mut input, ctx |\n            {\n                {\n                    let attrs = syn_grammar :: builtins ::\n                    parse_outer_attrs_impl(& mut input, ctx) ? ; let _t =\n                    input.parse :: < Token! [type] > () ? ;\n                    ctx.record_span(syn :: spanned :: Spanned :: span(& _t)) ? ;\n                    let name = syn_grammar :: builtins ::\n                    parse_ident_impl(& mut input, ctx) ? ; let (generics) = if\n                    let Some(vals) = rt ::\n                    attempt(input, ctx, | mut input, ctx |\n                    {\n                        let generics =\n                        {\n                            let start_span = input.span(); eprintln!\n                            (\"[TRACE] Attempting to parse syn type: {} at {:?}\",\n                            stringify! (syn :: Generics), start_span); match input.parse\n                            :: < syn :: Generics > ()\n                            {\n                                Ok(val) =>\n                                {\n                                    eprintln!\n                                    (\"[TRACE] Successfully parsed syn type: {}\", stringify!\n                                    (syn :: Generics)); Ok(val)\n                                }, Err(e) =>\n                                {\n                                    eprintln!\n                                    (\"[TRACE] FAILED to parse syn type: {}. Error: '{}', Error Span: {:?}\",\n                                    stringify! (syn :: Generics), e.to_string(), e.span());\n                                    eprintln! (\"[TRACE] Start span was: {:?}\", start_span); if\n                                    e.span().start() > start_span.start()\n                                    {\n                                        eprintln!\n                                        (\"[TRACE] Progress was made. Calling trigger_fail()\");\n                                        ctx.trigger_fail();\n                                    } else\n                                    {\n                                        eprintln!\n                                        (\"[TRACE] No progress. NOT calling trigger_fail()\");\n                                    } Err(e)\n                                }\n                            }\n                        } ? ; Ok((generics))\n                    }) ? { let (generics) = vals; (Some(generics)) } else\n                    { (None) }; let _t = input.parse :: < Token! [;] > () ? ;\n                    ctx.record_span(syn :: spanned :: Spanned :: span(& _t)) ? ;\n                    Ok({\n                        CxxItem ::\n                        Type(attrs, name.into(), generics.unwrap_or_default())\n                    })\n                }\n            }) ? { return Ok(res); }\n        }\n        {\n            if let Some(res) = rt ::\n            attempt_labeled(input, ctx, None, | mut input, ctx |\n            {\n                {\n                    let attrs = syn_grammar :: builtins ::\n                    parse_outer_attrs_impl(& mut input, ctx) ? ; let _t =\n                    input.parse :: < Token! [fn] > () ? ;\n                    ctx.record_span(syn :: spanned :: Spanned :: span(& _t)) ? ;\n                    let name = syn_grammar :: builtins ::\n                    parse_ident_impl(& mut input, ctx) ? ; let (generics) = if\n                    let Some(vals) = rt ::\n                    attempt(input, ctx, | mut input, ctx |\n                    {\n                        let generics =\n                        {\n                            let start_span = input.span(); eprintln!\n                            (\"[TRACE] Attempting to parse syn type: {} at {:?}\",\n                            stringify! (syn :: Generics), start_span); match input.parse\n                            :: < syn :: Generics > ()\n                            {\n                                Ok(val) =>\n                                {\n                                    eprintln!\n                                    (\"[TRACE] Successfully parsed syn type: {}\", stringify!\n                                    (syn :: Generics)); Ok(val)\n                                }, Err(e) =>\n                                {\n                                    eprintln!\n                                    (\"[TRACE] FAILED to parse syn type: {}. Error: '{}', Error Span: {:?}\",\n                                    stringify! (syn :: Generics), e.to_string(), e.span());\n                                    eprintln! (\"[TRACE] Start span was: {:?}\", start_span); if\n                                    e.span().start() > start_span.start()\n                                    {\n                                        eprintln!\n                                        (\"[TRACE] Progress was made. Calling trigger_fail()\");\n                                        ctx.trigger_fail();\n                                    } else\n                                    {\n                                        eprintln!\n                                        (\"[TRACE] No progress. NOT calling trigger_fail()\");\n                                    } Err(e)\n                                }\n                            }\n                        } ? ; Ok((generics))\n                    }) ? { let (generics) = vals; (Some(generics)) } else\n                    { (None) }; let args = rt ::\n                    parse_delimited(input, ctx, | mut input, ctx |\n                    {\n                        let (args) = if let Some(vals) = rt ::\n                        attempt(input, ctx, | mut input, ctx |\n                        {\n                            let args = parse_cxx_arg_list_impl(& mut input, ctx) ? ;\n                            Ok((args))\n                        }) ? { let (args) = vals; (Some(args)) } else { (None) };\n                        Ok(args)\n                    }, '(') ? ; let ret =\n                    {\n                        let start_span = input.span(); eprintln!\n                        (\"[TRACE] Attempting to parse syn type: {} at {:?}\",\n                        stringify! (syn :: ReturnType), start_span); match\n                        input.parse :: < syn :: ReturnType > ()\n                        {\n                            Ok(val) =>\n                            {\n                                eprintln!\n                                (\"[TRACE] Successfully parsed syn type: {}\", stringify!\n                                (syn :: ReturnType)); Ok(val)\n                            }, Err(e) =>\n                            {\n                                eprintln!\n                                (\"[TRACE] FAILED to parse syn type: {}. Error: '{}', Error Span: {:?}\",\n                                stringify! (syn :: ReturnType), e.to_string(), e.span());\n                                eprintln! (\"[TRACE] Start span was: {:?}\", start_span); if\n                                e.span().start() > start_span.start()\n                                {\n                                    eprintln!\n                                    (\"[TRACE] Progress was made. Calling trigger_fail()\");\n                                    ctx.trigger_fail();\n                                } else\n                                {\n                                    eprintln!\n                                    (\"[TRACE] No progress. NOT calling trigger_fail()\");\n                                } Err(e)\n                            }\n                        }\n                    } ? ; let _t = input.parse :: < Token! [;] > () ? ;\n                    ctx.record_span(syn :: spanned :: Spanned :: span(& _t)) ? ;\n                    Ok({\n                        CxxItem ::\n                        Function(attrs, name.into(), generics.unwrap_or_default(),\n                        args.unwrap_or_default(), ret)\n                    })\n                }\n            }) ? { return Ok(res); }\n        }\n        {\n            if let Some(res) = rt ::\n            attempt_labeled(input, ctx, None, | mut input, ctx |\n            {\n                {\n                    let mac =\n                    {\n                        let start_span = input.span(); eprintln!\n                        (\"[TRACE] Attempting to parse syn type: {} at {:?}\",\n                        stringify! (syn :: Macro), start_span); match input.parse ::\n                        < syn :: Macro > ()\n                        {\n                            Ok(val) =>\n                            {\n                                eprintln!\n                                (\"[TRACE] Successfully parsed syn type: {}\", stringify!\n                                (syn :: Macro)); Ok(val)\n                            }, Err(e) =>\n                            {\n                                eprintln!\n                                (\"[TRACE] FAILED to parse syn type: {}. Error: '{}', Error Span: {:?}\",\n                                stringify! (syn :: Macro), e.to_string(), e.span());\n                                eprintln! (\"[TRACE] Start span was: {:?}\", start_span); if\n                                e.span().start() > start_span.start()\n                                {\n                                    eprintln!\n                                    (\"[TRACE] Progress was made. Calling trigger_fail()\");\n                                    ctx.trigger_fail();\n                                } else\n                                {\n                                    eprintln!\n                                    (\"[TRACE] No progress. NOT calling trigger_fail()\");\n                                } Err(e)\n                            }\n                        }\n                    } ? ; let _t = input.parse :: < Token! [;] > () ? ;\n                    ctx.record_span(syn :: spanned :: Spanned :: span(& _t)) ? ;\n                    Ok({ CxxItem :: Macro(mac) })\n                }\n            }) ? { return Ok(res); }\n        } if ctx.stop_aggregation(_start_span)\n        {\n            if let Some(best_err) = ctx.take_best_error()\n            { return Err(best_err); }\n        } let mut error_to_return = if ! _shallow_failures.is_empty()\n        {\n            _shallow_failures.sort(); _shallow_failures.dedup(); let msg = if\n            _shallow_failures.len() == 1\n            { format! (\"expected `{}`\", _shallow_failures [0]) } else\n            {\n                let joined =\n                _shallow_failures.iter().map(| s | format!\n                (\"`{}`\", s)).collect :: < Vec < _ >> ().join(\", \"); format!\n                (\"expected one of: {}\", joined)\n            }; let _ = ctx.take_best_error(); input.error(msg)\n        } else if let Some(best_err) = ctx.take_best_error() { best_err } else\n        { input.error(\"No matching rule variant found\") }; if !\n        input.is_empty()\n        {\n            if let Ok(tt) = input.fork().parse :: < proc_macro2 :: TokenTree >\n            ()\n            {\n                let found = tt.to_string(); if ! found.trim().is_empty()\n                {\n                    let new_message = format!\n                    (\"{}; found unexpected token `{}`\", error_to_return, found);\n                    error_to_return = syn :: Error ::\n                    new(error_to_return.span(), new_message);\n                }\n            }\n        } Err(error_to_return)\n    }) (); if let Err(ref e) = res\n    { ctx.record_error(e.clone(), _start_span, None, 0); } ctx.exit_rule();\n    match res { Ok(val) => Ok(val), Err(e) => { Err(e) } }\n} #[doc = \"Parser for rule `extern_block`.\"] fn\nparse_extern_block(input : ParseStream) -> Result < ExternBlock >\n{\n    let mut ctx = rt :: ParseContext :: new(); match\n    parse_extern_block_impl(input, & mut ctx)\n    {\n        Ok(val) => Ok(val), Err(e) =>\n        {\n            if let Some(best) = ctx.take_best_error() { Err(best) } else\n            { Err(e) }\n        }\n    }\n} #[doc(hidden)] pub fn\nparse_extern_block_impl(mut input : ParseStream, ctx : & mut rt ::\nParseContext) -> Result < ExternBlock >\n{\n    ctx.enter_rule(stringify! (extern_block)); let _start_span = input.span();\n    let res =\n    (|| -> syn :: Result < ExternBlock >\n    {\n        let mut _shallow_failures = Vec :: < & str > :: new(); let _start_span\n        = input.span();\n        {\n            if let Some(res) = rt ::\n            attempt_labeled(input, ctx, Some(\"unsafe\"), | mut input, ctx |\n            {\n                {\n                    let (is_unsafe) = if input.peek(Token! [unsafe])\n                    {\n                        if let Some(vals) = rt ::\n                        attempt(input, ctx, | mut input, ctx |\n                        {\n                            let is_unsafe = input.parse :: < Token! [unsafe] > () ? ;\n                            ctx.record_span(syn :: spanned :: Spanned ::\n                            span(& is_unsafe)) ? ; Ok((is_unsafe))\n                        }) ? { let (is_unsafe) = vals; (Some(is_unsafe)) } else\n                        { (None) }\n                    } else { (None) }; let _t = input.parse :: < Token! [extern]\n                    > () ? ;\n                    ctx.record_span(syn :: spanned :: Spanned :: span(& _t)) ? ;\n                    let lang = syn_grammar :: builtins ::\n                    parse_lit_str_impl(& mut input, ctx) ? ; let items = rt ::\n                    parse_delimited(input, ctx, | mut input, ctx |\n                    {\n                        let mut _vec_items = Vec :: new(); while let Some(vals) = rt\n                        ::\n                        attempt(input, ctx, | mut input, ctx |\n                        {\n                            let items = parse_cxx_item_impl(& mut input, ctx) ? ;\n                            Ok((items))\n                        }) ? { let (items) = vals; _vec_items.push(items); } let\n                        items = _vec_items; Ok(items)\n                    }, '{') ? ;\n                    Ok({\n                        ExternBlock\n                        {\n                            is_unsafe: is_unsafe.is_some(), lang: lang.into(), items,\n                        }\n                    })\n                }\n            }) ? { return Ok(res); }\n        } if ! ctx.stop_aggregation(input.span())\n        { _shallow_failures.push(\"unsafe\"); } if\n        ctx.stop_aggregation(_start_span)\n        {\n            if let Some(best_err) = ctx.take_best_error()\n            { return Err(best_err); }\n        } let mut error_to_return = if ! _shallow_failures.is_empty()\n        {\n            _shallow_failures.sort(); _shallow_failures.dedup(); let msg = if\n            _shallow_failures.len() == 1\n            { format! (\"expected `{}`\", _shallow_failures [0]) } else\n            {\n                let joined =\n                _shallow_failures.iter().map(| s | format!\n                (\"`{}`\", s)).collect :: < Vec < _ >> ().join(\", \"); format!\n                (\"expected one of: {}\", joined)\n            }; let _ = ctx.take_best_error(); input.error(msg)\n        } else if let Some(best_err) = ctx.take_best_error() { best_err } else\n        { input.error(\"No matching rule variant found\") }; if !\n        input.is_empty()\n        {\n            if let Ok(tt) = input.fork().parse :: < proc_macro2 :: TokenTree >\n            ()\n            {\n                let found = tt.to_string(); if ! found.trim().is_empty()\n                {\n                    let new_message = format!\n                    (\"{}; found unexpected token `{}`\", error_to_return, found);\n                    error_to_return = syn :: Error ::\n                    new(error_to_return.span(), new_message);\n                }\n            }\n        } Err(error_to_return)\n    }) (); if let Err(ref e) = res\n    { ctx.record_error(e.clone(), _start_span, None, 0); } ctx.exit_rule();\n    match res { Ok(val) => Ok(val), Err(e) => { Err(e) } }\n} #[doc = \"Parser for rule `top_level_mod`.\"] pub fn\nparse_top_level_mod(input : ParseStream) -> Result < FfiMod >\n{\n    let mut ctx = rt :: ParseContext :: new(); match\n    parse_top_level_mod_impl(input, & mut ctx)\n    {\n        Ok(val) => Ok(val), Err(e) =>\n        {\n            if let Some(best) = ctx.take_best_error() { Err(best) } else\n            { Err(e) }\n        }\n    }\n} #[doc(hidden)] pub fn\nparse_top_level_mod_impl(mut input : ParseStream, ctx : & mut rt ::\nParseContext) -> Result < FfiMod >\n{\n    ctx.enter_rule(stringify! (top_level_mod)); let _start_span =\n    input.span(); let res =\n    (|| -> syn :: Result < FfiMod >\n    {\n        let mut _shallow_failures = Vec :: < & str > :: new(); let _start_span\n        = input.span();\n        {\n            if let Some(res) = rt ::\n            attempt_labeled(input, ctx, None, | mut input, ctx |\n            {\n                {\n                    let attrs = syn_grammar :: builtins ::\n                    parse_outer_attrs_impl(& mut input, ctx) ? ; let _t =\n                    input.parse :: < Token! [mod] > () ? ;\n                    ctx.record_span(syn :: spanned :: Spanned :: span(& _t)) ? ;\n                    let name = syn_grammar :: builtins ::\n                    parse_ident_impl(& mut input, ctx) ? ; let blocks = rt ::\n                    parse_delimited(input, ctx, | mut input, ctx |\n                    {\n                        let mut _vec_blocks = Vec :: new(); while let Some(vals) =\n                        rt ::\n                        attempt(input, ctx, | mut input, ctx |\n                        {\n                            let blocks = parse_extern_block_impl(& mut input, ctx) ? ;\n                            Ok((blocks))\n                        }) ? { let (blocks) = vals; _vec_blocks.push(blocks); } let\n                        blocks = _vec_blocks; Ok(blocks)\n                    }, '{') ? ;\n                    Ok({ FfiMod { attrs, name: name.into(), blocks } })\n                }\n            }) ? { return Ok(res); }\n        } if ctx.stop_aggregation(_start_span)\n        {\n            if let Some(best_err) = ctx.take_best_error()\n            { return Err(best_err); }\n        } let mut error_to_return = if ! _shallow_failures.is_empty()\n        {\n            _shallow_failures.sort(); _shallow_failures.dedup(); let msg = if\n            _shallow_failures.len() == 1\n            { format! (\"expected `{}`\", _shallow_failures [0]) } else\n            {\n                let joined =\n                _shallow_failures.iter().map(| s | format!\n                (\"`{}`\", s)).collect :: < Vec < _ >> ().join(\", \"); format!\n                (\"expected one of: {}\", joined)\n            }; let _ = ctx.take_best_error(); input.error(msg)\n        } else if let Some(best_err) = ctx.take_best_error() { best_err } else\n        { input.error(\"No matching rule variant found\") }; if !\n        input.is_empty()\n        {\n            if let Ok(tt) = input.fork().parse :: < proc_macro2 :: TokenTree >\n            ()\n            {\n                let found = tt.to_string(); if ! found.trim().is_empty()\n                {\n                    let new_message = format!\n                    (\"{}; found unexpected token `{}`\", error_to_return, found);\n                    error_to_return = syn :: Error ::\n                    new(error_to_return.span(), new_message);\n                }\n            }\n        } Err(error_to_return)\n    }) (); if let Err(ref e) = res\n    { ctx.record_error(e.clone(), _start_span, None, 0); } ctx.exit_rule();\n    match res { Ok(val) => Ok(val), Err(e) => { Err(e) } }\n}";
    use super::*;
    use syn::parse::{Parse, ParseStream};
    use syn::Result;
    use syn::Token;
    use syn::ext::IdentExt;
    use syn::spanned::Spanned;
    use syn_grammar::rt;
    #[allow(unused_imports)]
    use syn_grammar::builtins::*;
    ///Parser for rule `cxx_arg`.
    fn parse_cxx_arg(input: ParseStream) -> Result<CxxArg> {
        let mut ctx = rt::ParseContext::new();
        match parse_cxx_arg_impl(input, &mut ctx) {
            Ok(val) => Ok(val),
            Err(e) => {
                if let Some(best) = ctx.take_best_error() { Err(best) } else { Err(e) }
            }
        }
    }
    #[doc(hidden)]
    pub fn parse_cxx_arg_impl(
        mut input: ParseStream,
        ctx: &mut rt::ParseContext,
    ) -> Result<CxxArg> {
        ctx.enter_rule("cxx_arg");
        let _start_span = input.span();
        let res = (|| -> syn::Result<CxxArg> {
            let mut _shallow_failures = Vec::<&str>::new();
            let _start_span = input.span();
            {
                if let Some(res) = rt::attempt_labeled(
                    input,
                    ctx,
                    None,
                    |mut input, ctx| {
                        {
                            let name = syn_grammar::builtins::parse_any_ident_impl(
                                &mut input,
                                ctx,
                            )?;
                            let _t = input.parse::<::syn::token::Colon>()?;
                            ctx.record_span(syn::spanned::Spanned::span(&_t))?;
                            let ty = {
                                let start_span = input.span();
                                {
                                    ::std::io::_eprint(
                                        format_args!(
                                            "[TRACE] Attempting to parse syn type: {0} at {1:?}\n",
                                            "syn :: Type",
                                            start_span,
                                        ),
                                    );
                                };
                                match input.parse::<syn::Type>() {
                                    Ok(val) => {
                                        {
                                            ::std::io::_eprint(
                                                format_args!(
                                                    "[TRACE] Successfully parsed syn type: {0}\n",
                                                    "syn :: Type",
                                                ),
                                            );
                                        };
                                        Ok(val)
                                    }
                                    Err(e) => {
                                        {
                                            ::std::io::_eprint(
                                                format_args!(
                                                    "[TRACE] FAILED to parse syn type: {0}. Error: \'{1}\', Error Span: {2:?}\n",
                                                    "syn :: Type",
                                                    e.to_string(),
                                                    e.span(),
                                                ),
                                            );
                                        };
                                        {
                                            ::std::io::_eprint(
                                                format_args!("[TRACE] Start span was: {0:?}\n", start_span),
                                            );
                                        };
                                        if e.span().start() > start_span.start() {
                                            {
                                                ::std::io::_eprint(
                                                    format_args!(
                                                        "[TRACE] Progress was made. Calling trigger_fail()\n",
                                                    ),
                                                );
                                            };
                                            ctx.trigger_fail();
                                        } else {
                                            {
                                                ::std::io::_eprint(
                                                    format_args!(
                                                        "[TRACE] No progress. NOT calling trigger_fail()\n",
                                                    ),
                                                );
                                            };
                                        }
                                        Err(e)
                                    }
                                }
                            }?;
                            Ok({ CxxArg { name: name.into(), ty } })
                        }
                    },
                )? {
                    return Ok(res);
                }
            }
            if ctx.stop_aggregation(_start_span) {
                if let Some(best_err) = ctx.take_best_error() {
                    return Err(best_err);
                }
            }
            let mut error_to_return = if !_shallow_failures.is_empty() {
                _shallow_failures.sort();
                _shallow_failures.dedup();
                let msg = if _shallow_failures.len() == 1 {
                    ::alloc::__export::must_use({
                        ::alloc::fmt::format(
                            format_args!("expected `{0}`", _shallow_failures[0]),
                        )
                    })
                } else {
                    let joined = _shallow_failures
                        .iter()
                        .map(|s| ::alloc::__export::must_use({
                            ::alloc::fmt::format(format_args!("`{0}`", s))
                        }))
                        .collect::<Vec<_>>()
                        .join(", ");
                    ::alloc::__export::must_use({
                        ::alloc::fmt::format(
                            format_args!("expected one of: {0}", joined),
                        )
                    })
                };
                let _ = ctx.take_best_error();
                input.error(msg)
            } else if let Some(best_err) = ctx.take_best_error() {
                best_err
            } else {
                input.error("No matching rule variant found")
            };
            if !input.is_empty() {
                if let Ok(tt) = input.fork().parse::<proc_macro2::TokenTree>() {
                    let found = tt.to_string();
                    if !found.trim().is_empty() {
                        let new_message = ::alloc::__export::must_use({
                            ::alloc::fmt::format(
                                format_args!(
                                    "{0}; found unexpected token `{1}`",
                                    error_to_return,
                                    found,
                                ),
                            )
                        });
                        error_to_return = syn::Error::new(
                            error_to_return.span(),
                            new_message,
                        );
                    }
                }
            }
            Err(error_to_return)
        })();
        if let Err(ref e) = res {
            ctx.record_error(e.clone(), _start_span, None, 0);
        }
        ctx.exit_rule();
        match res {
            Ok(val) => Ok(val),
            Err(e) => Err(e),
        }
    }
    ///Parser for rule `cxx_arg_list`.
    fn parse_cxx_arg_list(input: ParseStream) -> Result<Vec<CxxArg>> {
        let mut ctx = rt::ParseContext::new();
        match parse_cxx_arg_list_impl(input, &mut ctx) {
            Ok(val) => Ok(val),
            Err(e) => {
                if let Some(best) = ctx.take_best_error() { Err(best) } else { Err(e) }
            }
        }
    }
    #[doc(hidden)]
    pub fn parse_cxx_arg_list_impl(
        mut input: ParseStream,
        ctx: &mut rt::ParseContext,
    ) -> Result<Vec<CxxArg>> {
        ctx.enter_rule("cxx_arg_list");
        let _start_span = input.span();
        let res = (|| -> syn::Result<Vec<CxxArg>> {
            let mut _shallow_failures = Vec::<&str>::new();
            let _start_span = input.span();
            {
                if let Some(res) = rt::attempt_labeled(
                    input,
                    ctx,
                    Some("separated"),
                    |mut input, ctx| {
                        {
                            let items = {
                                let _items_vec = rt::parse_separated::<
                                    _,
                                    _,
                                    _,
                                >(
                                    input,
                                    ctx,
                                    |mut input, ctx| {
                                        let _item = parse_cxx_arg_impl(&mut input, ctx)?;
                                        Ok(_item)
                                    },
                                    |mut input, ctx| {
                                        let _t = input.parse::<::syn::token::Comma>()?;
                                        ctx.record_span(syn::spanned::Spanned::span(&_t))?;
                                        Ok(())
                                    },
                                    0usize,
                                    false,
                                    None,
                                )?;
                                let mut _items = Vec::from_iter(_items_vec);
                                _items
                            };
                            Ok({ items })
                        }
                    },
                )? {
                    return Ok(res);
                }
            }
            if !ctx.stop_aggregation(input.span()) {
                _shallow_failures.push("separated");
            }
            if ctx.stop_aggregation(_start_span) {
                if let Some(best_err) = ctx.take_best_error() {
                    return Err(best_err);
                }
            }
            let mut error_to_return = if !_shallow_failures.is_empty() {
                _shallow_failures.sort();
                _shallow_failures.dedup();
                let msg = if _shallow_failures.len() == 1 {
                    ::alloc::__export::must_use({
                        ::alloc::fmt::format(
                            format_args!("expected `{0}`", _shallow_failures[0]),
                        )
                    })
                } else {
                    let joined = _shallow_failures
                        .iter()
                        .map(|s| ::alloc::__export::must_use({
                            ::alloc::fmt::format(format_args!("`{0}`", s))
                        }))
                        .collect::<Vec<_>>()
                        .join(", ");
                    ::alloc::__export::must_use({
                        ::alloc::fmt::format(
                            format_args!("expected one of: {0}", joined),
                        )
                    })
                };
                let _ = ctx.take_best_error();
                input.error(msg)
            } else if let Some(best_err) = ctx.take_best_error() {
                best_err
            } else {
                input.error("No matching rule variant found")
            };
            if !input.is_empty() {
                if let Ok(tt) = input.fork().parse::<proc_macro2::TokenTree>() {
                    let found = tt.to_string();
                    if !found.trim().is_empty() {
                        let new_message = ::alloc::__export::must_use({
                            ::alloc::fmt::format(
                                format_args!(
                                    "{0}; found unexpected token `{1}`",
                                    error_to_return,
                                    found,
                                ),
                            )
                        });
                        error_to_return = syn::Error::new(
                            error_to_return.span(),
                            new_message,
                        );
                    }
                }
            }
            Err(error_to_return)
        })();
        if let Err(ref e) = res {
            ctx.record_error(e.clone(), _start_span, None, 0);
        }
        ctx.exit_rule();
        match res {
            Ok(val) => Ok(val),
            Err(e) => Err(e),
        }
    }
    ///Parser for rule `cxx_item`.
    fn parse_cxx_item(input: ParseStream) -> Result<CxxItem> {
        let mut ctx = rt::ParseContext::new();
        match parse_cxx_item_impl(input, &mut ctx) {
            Ok(val) => Ok(val),
            Err(e) => {
                if let Some(best) = ctx.take_best_error() { Err(best) } else { Err(e) }
            }
        }
    }
    #[doc(hidden)]
    pub fn parse_cxx_item_impl(
        mut input: ParseStream,
        ctx: &mut rt::ParseContext,
    ) -> Result<CxxItem> {
        ctx.enter_rule("cxx_item");
        let _start_span = input.span();
        let res = (|| -> syn::Result<CxxItem> {
            let mut _shallow_failures = Vec::<&str>::new();
            let _start_span = input.span();
            {
                if let Some(res) = rt::attempt_labeled(
                    input,
                    ctx,
                    None,
                    |mut input, ctx| {
                        {
                            let attrs = syn_grammar::builtins::parse_outer_attrs_impl(
                                &mut input,
                                ctx,
                            )?;
                            let _t = input.parse::<::syn::token::Type>()?;
                            ctx.record_span(syn::spanned::Spanned::span(&_t))?;
                            let name = syn_grammar::builtins::parse_ident_impl(
                                &mut input,
                                ctx,
                            )?;
                            let (generics) = if let Some(vals) = rt::attempt(
                                input,
                                ctx,
                                |mut input, ctx| {
                                    let generics = {
                                        let start_span = input.span();
                                        {
                                            ::std::io::_eprint(
                                                format_args!(
                                                    "[TRACE] Attempting to parse syn type: {0} at {1:?}\n",
                                                    "syn :: Generics",
                                                    start_span,
                                                ),
                                            );
                                        };
                                        match input.parse::<syn::Generics>() {
                                            Ok(val) => {
                                                {
                                                    ::std::io::_eprint(
                                                        format_args!(
                                                            "[TRACE] Successfully parsed syn type: {0}\n",
                                                            "syn :: Generics",
                                                        ),
                                                    );
                                                };
                                                Ok(val)
                                            }
                                            Err(e) => {
                                                {
                                                    ::std::io::_eprint(
                                                        format_args!(
                                                            "[TRACE] FAILED to parse syn type: {0}. Error: \'{1}\', Error Span: {2:?}\n",
                                                            "syn :: Generics",
                                                            e.to_string(),
                                                            e.span(),
                                                        ),
                                                    );
                                                };
                                                {
                                                    ::std::io::_eprint(
                                                        format_args!("[TRACE] Start span was: {0:?}\n", start_span),
                                                    );
                                                };
                                                if e.span().start() > start_span.start() {
                                                    {
                                                        ::std::io::_eprint(
                                                            format_args!(
                                                                "[TRACE] Progress was made. Calling trigger_fail()\n",
                                                            ),
                                                        );
                                                    };
                                                    ctx.trigger_fail();
                                                } else {
                                                    {
                                                        ::std::io::_eprint(
                                                            format_args!(
                                                                "[TRACE] No progress. NOT calling trigger_fail()\n",
                                                            ),
                                                        );
                                                    };
                                                }
                                                Err(e)
                                            }
                                        }
                                    }?;
                                    Ok((generics))
                                },
                            )? {
                                let (generics) = vals;
                                (Some(generics))
                            } else {
                                (None)
                            };
                            let _t = input.parse::<::syn::token::Semi>()?;
                            ctx.record_span(syn::spanned::Spanned::span(&_t))?;
                            Ok({
                                CxxItem::Type(
                                    attrs,
                                    name.into(),
                                    generics.unwrap_or_default(),
                                )
                            })
                        }
                    },
                )? {
                    return Ok(res);
                }
            }
            {
                if let Some(res) = rt::attempt_labeled(
                    input,
                    ctx,
                    None,
                    |mut input, ctx| {
                        {
                            let attrs = syn_grammar::builtins::parse_outer_attrs_impl(
                                &mut input,
                                ctx,
                            )?;
                            let _t = input.parse::<::syn::token::Fn>()?;
                            ctx.record_span(syn::spanned::Spanned::span(&_t))?;
                            let name = syn_grammar::builtins::parse_ident_impl(
                                &mut input,
                                ctx,
                            )?;
                            let (generics) = if let Some(vals) = rt::attempt(
                                input,
                                ctx,
                                |mut input, ctx| {
                                    let generics = {
                                        let start_span = input.span();
                                        {
                                            ::std::io::_eprint(
                                                format_args!(
                                                    "[TRACE] Attempting to parse syn type: {0} at {1:?}\n",
                                                    "syn :: Generics",
                                                    start_span,
                                                ),
                                            );
                                        };
                                        match input.parse::<syn::Generics>() {
                                            Ok(val) => {
                                                {
                                                    ::std::io::_eprint(
                                                        format_args!(
                                                            "[TRACE] Successfully parsed syn type: {0}\n",
                                                            "syn :: Generics",
                                                        ),
                                                    );
                                                };
                                                Ok(val)
                                            }
                                            Err(e) => {
                                                {
                                                    ::std::io::_eprint(
                                                        format_args!(
                                                            "[TRACE] FAILED to parse syn type: {0}. Error: \'{1}\', Error Span: {2:?}\n",
                                                            "syn :: Generics",
                                                            e.to_string(),
                                                            e.span(),
                                                        ),
                                                    );
                                                };
                                                {
                                                    ::std::io::_eprint(
                                                        format_args!("[TRACE] Start span was: {0:?}\n", start_span),
                                                    );
                                                };
                                                if e.span().start() > start_span.start() {
                                                    {
                                                        ::std::io::_eprint(
                                                            format_args!(
                                                                "[TRACE] Progress was made. Calling trigger_fail()\n",
                                                            ),
                                                        );
                                                    };
                                                    ctx.trigger_fail();
                                                } else {
                                                    {
                                                        ::std::io::_eprint(
                                                            format_args!(
                                                                "[TRACE] No progress. NOT calling trigger_fail()\n",
                                                            ),
                                                        );
                                                    };
                                                }
                                                Err(e)
                                            }
                                        }
                                    }?;
                                    Ok((generics))
                                },
                            )? {
                                let (generics) = vals;
                                (Some(generics))
                            } else {
                                (None)
                            };
                            let args = rt::parse_delimited(
                                input,
                                ctx,
                                |mut input, ctx| {
                                    let (args) = if let Some(vals) = rt::attempt(
                                        input,
                                        ctx,
                                        |mut input, ctx| {
                                            let args = parse_cxx_arg_list_impl(&mut input, ctx)?;
                                            Ok((args))
                                        },
                                    )? {
                                        let (args) = vals;
                                        (Some(args))
                                    } else {
                                        (None)
                                    };
                                    Ok(args)
                                },
                                '(',
                            )?;
                            let ret = {
                                let start_span = input.span();
                                {
                                    ::std::io::_eprint(
                                        format_args!(
                                            "[TRACE] Attempting to parse syn type: {0} at {1:?}\n",
                                            "syn :: ReturnType",
                                            start_span,
                                        ),
                                    );
                                };
                                match input.parse::<syn::ReturnType>() {
                                    Ok(val) => {
                                        {
                                            ::std::io::_eprint(
                                                format_args!(
                                                    "[TRACE] Successfully parsed syn type: {0}\n",
                                                    "syn :: ReturnType",
                                                ),
                                            );
                                        };
                                        Ok(val)
                                    }
                                    Err(e) => {
                                        {
                                            ::std::io::_eprint(
                                                format_args!(
                                                    "[TRACE] FAILED to parse syn type: {0}. Error: \'{1}\', Error Span: {2:?}\n",
                                                    "syn :: ReturnType",
                                                    e.to_string(),
                                                    e.span(),
                                                ),
                                            );
                                        };
                                        {
                                            ::std::io::_eprint(
                                                format_args!("[TRACE] Start span was: {0:?}\n", start_span),
                                            );
                                        };
                                        if e.span().start() > start_span.start() {
                                            {
                                                ::std::io::_eprint(
                                                    format_args!(
                                                        "[TRACE] Progress was made. Calling trigger_fail()\n",
                                                    ),
                                                );
                                            };
                                            ctx.trigger_fail();
                                        } else {
                                            {
                                                ::std::io::_eprint(
                                                    format_args!(
                                                        "[TRACE] No progress. NOT calling trigger_fail()\n",
                                                    ),
                                                );
                                            };
                                        }
                                        Err(e)
                                    }
                                }
                            }?;
                            let _t = input.parse::<::syn::token::Semi>()?;
                            ctx.record_span(syn::spanned::Spanned::span(&_t))?;
                            Ok({
                                CxxItem::Function(
                                    attrs,
                                    name.into(),
                                    generics.unwrap_or_default(),
                                    args.unwrap_or_default(),
                                    ret,
                                )
                            })
                        }
                    },
                )? {
                    return Ok(res);
                }
            }
            {
                if let Some(res) = rt::attempt_labeled(
                    input,
                    ctx,
                    None,
                    |mut input, ctx| {
                        {
                            let mac = {
                                let start_span = input.span();
                                {
                                    ::std::io::_eprint(
                                        format_args!(
                                            "[TRACE] Attempting to parse syn type: {0} at {1:?}\n",
                                            "syn :: Macro",
                                            start_span,
                                        ),
                                    );
                                };
                                match input.parse::<syn::Macro>() {
                                    Ok(val) => {
                                        {
                                            ::std::io::_eprint(
                                                format_args!(
                                                    "[TRACE] Successfully parsed syn type: {0}\n",
                                                    "syn :: Macro",
                                                ),
                                            );
                                        };
                                        Ok(val)
                                    }
                                    Err(e) => {
                                        {
                                            ::std::io::_eprint(
                                                format_args!(
                                                    "[TRACE] FAILED to parse syn type: {0}. Error: \'{1}\', Error Span: {2:?}\n",
                                                    "syn :: Macro",
                                                    e.to_string(),
                                                    e.span(),
                                                ),
                                            );
                                        };
                                        {
                                            ::std::io::_eprint(
                                                format_args!("[TRACE] Start span was: {0:?}\n", start_span),
                                            );
                                        };
                                        if e.span().start() > start_span.start() {
                                            {
                                                ::std::io::_eprint(
                                                    format_args!(
                                                        "[TRACE] Progress was made. Calling trigger_fail()\n",
                                                    ),
                                                );
                                            };
                                            ctx.trigger_fail();
                                        } else {
                                            {
                                                ::std::io::_eprint(
                                                    format_args!(
                                                        "[TRACE] No progress. NOT calling trigger_fail()\n",
                                                    ),
                                                );
                                            };
                                        }
                                        Err(e)
                                    }
                                }
                            }?;
                            let _t = input.parse::<::syn::token::Semi>()?;
                            ctx.record_span(syn::spanned::Spanned::span(&_t))?;
                            Ok({ CxxItem::Macro(mac) })
                        }
                    },
                )? {
                    return Ok(res);
                }
            }
            if ctx.stop_aggregation(_start_span) {
                if let Some(best_err) = ctx.take_best_error() {
                    return Err(best_err);
                }
            }
            let mut error_to_return = if !_shallow_failures.is_empty() {
                _shallow_failures.sort();
                _shallow_failures.dedup();
                let msg = if _shallow_failures.len() == 1 {
                    ::alloc::__export::must_use({
                        ::alloc::fmt::format(
                            format_args!("expected `{0}`", _shallow_failures[0]),
                        )
                    })
                } else {
                    let joined = _shallow_failures
                        .iter()
                        .map(|s| ::alloc::__export::must_use({
                            ::alloc::fmt::format(format_args!("`{0}`", s))
                        }))
                        .collect::<Vec<_>>()
                        .join(", ");
                    ::alloc::__export::must_use({
                        ::alloc::fmt::format(
                            format_args!("expected one of: {0}", joined),
                        )
                    })
                };
                let _ = ctx.take_best_error();
                input.error(msg)
            } else if let Some(best_err) = ctx.take_best_error() {
                best_err
            } else {
                input.error("No matching rule variant found")
            };
            if !input.is_empty() {
                if let Ok(tt) = input.fork().parse::<proc_macro2::TokenTree>() {
                    let found = tt.to_string();
                    if !found.trim().is_empty() {
                        let new_message = ::alloc::__export::must_use({
                            ::alloc::fmt::format(
                                format_args!(
                                    "{0}; found unexpected token `{1}`",
                                    error_to_return,
                                    found,
                                ),
                            )
                        });
                        error_to_return = syn::Error::new(
                            error_to_return.span(),
                            new_message,
                        );
                    }
                }
            }
            Err(error_to_return)
        })();
        if let Err(ref e) = res {
            ctx.record_error(e.clone(), _start_span, None, 0);
        }
        ctx.exit_rule();
        match res {
            Ok(val) => Ok(val),
            Err(e) => Err(e),
        }
    }
    ///Parser for rule `extern_block`.
    fn parse_extern_block(input: ParseStream) -> Result<ExternBlock> {
        let mut ctx = rt::ParseContext::new();
        match parse_extern_block_impl(input, &mut ctx) {
            Ok(val) => Ok(val),
            Err(e) => {
                if let Some(best) = ctx.take_best_error() { Err(best) } else { Err(e) }
            }
        }
    }
    #[doc(hidden)]
    pub fn parse_extern_block_impl(
        mut input: ParseStream,
        ctx: &mut rt::ParseContext,
    ) -> Result<ExternBlock> {
        ctx.enter_rule("extern_block");
        let _start_span = input.span();
        let res = (|| -> syn::Result<ExternBlock> {
            let mut _shallow_failures = Vec::<&str>::new();
            let _start_span = input.span();
            {
                if let Some(res) = rt::attempt_labeled(
                    input,
                    ctx,
                    Some("unsafe"),
                    |mut input, ctx| {
                        {
                            let (is_unsafe) = if input.peek(::syn::token::Unsafe) {
                                if let Some(vals) = rt::attempt(
                                    input,
                                    ctx,
                                    |mut input, ctx| {
                                        let is_unsafe = input.parse::<::syn::token::Unsafe>()?;
                                        ctx.record_span(syn::spanned::Spanned::span(&is_unsafe))?;
                                        Ok((is_unsafe))
                                    },
                                )? {
                                    let (is_unsafe) = vals;
                                    (Some(is_unsafe))
                                } else {
                                    (None)
                                }
                            } else {
                                (None)
                            };
                            let _t = input.parse::<::syn::token::Extern>()?;
                            ctx.record_span(syn::spanned::Spanned::span(&_t))?;
                            let lang = syn_grammar::builtins::parse_lit_str_impl(
                                &mut input,
                                ctx,
                            )?;
                            let items = rt::parse_delimited(
                                input,
                                ctx,
                                |mut input, ctx| {
                                    let mut _vec_items = Vec::new();
                                    while let Some(vals) = rt::attempt(
                                        input,
                                        ctx,
                                        |mut input, ctx| {
                                            let items = parse_cxx_item_impl(&mut input, ctx)?;
                                            Ok((items))
                                        },
                                    )? {
                                        let (items) = vals;
                                        _vec_items.push(items);
                                    }
                                    let items = _vec_items;
                                    Ok(items)
                                },
                                '{',
                            )?;
                            Ok({
                                ExternBlock {
                                    is_unsafe: is_unsafe.is_some(),
                                    lang: lang.into(),
                                    items,
                                }
                            })
                        }
                    },
                )? {
                    return Ok(res);
                }
            }
            if !ctx.stop_aggregation(input.span()) {
                _shallow_failures.push("unsafe");
            }
            if ctx.stop_aggregation(_start_span) {
                if let Some(best_err) = ctx.take_best_error() {
                    return Err(best_err);
                }
            }
            let mut error_to_return = if !_shallow_failures.is_empty() {
                _shallow_failures.sort();
                _shallow_failures.dedup();
                let msg = if _shallow_failures.len() == 1 {
                    ::alloc::__export::must_use({
                        ::alloc::fmt::format(
                            format_args!("expected `{0}`", _shallow_failures[0]),
                        )
                    })
                } else {
                    let joined = _shallow_failures
                        .iter()
                        .map(|s| ::alloc::__export::must_use({
                            ::alloc::fmt::format(format_args!("`{0}`", s))
                        }))
                        .collect::<Vec<_>>()
                        .join(", ");
                    ::alloc::__export::must_use({
                        ::alloc::fmt::format(
                            format_args!("expected one of: {0}", joined),
                        )
                    })
                };
                let _ = ctx.take_best_error();
                input.error(msg)
            } else if let Some(best_err) = ctx.take_best_error() {
                best_err
            } else {
                input.error("No matching rule variant found")
            };
            if !input.is_empty() {
                if let Ok(tt) = input.fork().parse::<proc_macro2::TokenTree>() {
                    let found = tt.to_string();
                    if !found.trim().is_empty() {
                        let new_message = ::alloc::__export::must_use({
                            ::alloc::fmt::format(
                                format_args!(
                                    "{0}; found unexpected token `{1}`",
                                    error_to_return,
                                    found,
                                ),
                            )
                        });
                        error_to_return = syn::Error::new(
                            error_to_return.span(),
                            new_message,
                        );
                    }
                }
            }
            Err(error_to_return)
        })();
        if let Err(ref e) = res {
            ctx.record_error(e.clone(), _start_span, None, 0);
        }
        ctx.exit_rule();
        match res {
            Ok(val) => Ok(val),
            Err(e) => Err(e),
        }
    }
    ///Parser for rule `top_level_mod`.
    pub fn parse_top_level_mod(input: ParseStream) -> Result<FfiMod> {
        let mut ctx = rt::ParseContext::new();
        match parse_top_level_mod_impl(input, &mut ctx) {
            Ok(val) => Ok(val),
            Err(e) => {
                if let Some(best) = ctx.take_best_error() { Err(best) } else { Err(e) }
            }
        }
    }
    #[doc(hidden)]
    pub fn parse_top_level_mod_impl(
        mut input: ParseStream,
        ctx: &mut rt::ParseContext,
    ) -> Result<FfiMod> {
        ctx.enter_rule("top_level_mod");
        let _start_span = input.span();
        let res = (|| -> syn::Result<FfiMod> {
            let mut _shallow_failures = Vec::<&str>::new();
            let _start_span = input.span();
            {
                if let Some(res) = rt::attempt_labeled(
                    input,
                    ctx,
                    None,
                    |mut input, ctx| {
                        {
                            let attrs = syn_grammar::builtins::parse_outer_attrs_impl(
                                &mut input,
                                ctx,
                            )?;
                            let _t = input.parse::<::syn::token::Mod>()?;
                            ctx.record_span(syn::spanned::Spanned::span(&_t))?;
                            let name = syn_grammar::builtins::parse_ident_impl(
                                &mut input,
                                ctx,
                            )?;
                            let blocks = rt::parse_delimited(
                                input,
                                ctx,
                                |mut input, ctx| {
                                    let mut _vec_blocks = Vec::new();
                                    while let Some(vals) = rt::attempt(
                                        input,
                                        ctx,
                                        |mut input, ctx| {
                                            let blocks = parse_extern_block_impl(&mut input, ctx)?;
                                            Ok((blocks))
                                        },
                                    )? {
                                        let (blocks) = vals;
                                        _vec_blocks.push(blocks);
                                    }
                                    let blocks = _vec_blocks;
                                    Ok(blocks)
                                },
                                '{',
                            )?;
                            Ok({
                                FfiMod {
                                    attrs,
                                    name: name.into(),
                                    blocks,
                                }
                            })
                        }
                    },
                )? {
                    return Ok(res);
                }
            }
            if ctx.stop_aggregation(_start_span) {
                if let Some(best_err) = ctx.take_best_error() {
                    return Err(best_err);
                }
            }
            let mut error_to_return = if !_shallow_failures.is_empty() {
                _shallow_failures.sort();
                _shallow_failures.dedup();
                let msg = if _shallow_failures.len() == 1 {
                    ::alloc::__export::must_use({
                        ::alloc::fmt::format(
                            format_args!("expected `{0}`", _shallow_failures[0]),
                        )
                    })
                } else {
                    let joined = _shallow_failures
                        .iter()
                        .map(|s| ::alloc::__export::must_use({
                            ::alloc::fmt::format(format_args!("`{0}`", s))
                        }))
                        .collect::<Vec<_>>()
                        .join(", ");
                    ::alloc::__export::must_use({
                        ::alloc::fmt::format(
                            format_args!("expected one of: {0}", joined),
                        )
                    })
                };
                let _ = ctx.take_best_error();
                input.error(msg)
            } else if let Some(best_err) = ctx.take_best_error() {
                best_err
            } else {
                input.error("No matching rule variant found")
            };
            if !input.is_empty() {
                if let Ok(tt) = input.fork().parse::<proc_macro2::TokenTree>() {
                    let found = tt.to_string();
                    if !found.trim().is_empty() {
                        let new_message = ::alloc::__export::must_use({
                            ::alloc::fmt::format(
                                format_args!(
                                    "{0}; found unexpected token `{1}`",
                                    error_to_return,
                                    found,
                                ),
                            )
                        });
                        error_to_return = syn::Error::new(
                            error_to_return.span(),
                            new_message,
                        );
                    }
                }
            }
            Err(error_to_return)
        })();
        if let Err(ref e) = res {
            ctx.record_error(e.clone(), _start_span, None, 0);
        }
        ctx.exit_rule();
        match res {
            Ok(val) => Ok(val),
            Err(e) => Err(e),
        }
    }
}
impl Parse for FfiMod {
    fn parse(input: ParseStream) -> Result<Self> {
        CxxParser::parse_top_level_mod(input)
    }
}
