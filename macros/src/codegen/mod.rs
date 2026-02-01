mod pattern;
mod rule;

use syn_grammar_model::{model::*, analysis};
use quote::{quote, format_ident};
use proc_macro2::TokenStream;
use syn::Result;

pub fn generate_rust(grammar: GrammarDefinition) -> Result<TokenStream> {
    let grammar_name = &grammar.name;
    let custom_keywords = analysis::collect_custom_keywords(&grammar);

    let kw_defs = (!custom_keywords.is_empty()).then(|| {
        let defs = custom_keywords.iter().map(|k| {
            let ident = format_ident!("{}", k);
            quote! { syn::custom_keyword!(#ident); }
        });
        quote! { pub mod kw { #(#defs)* } }
    });

    let inheritance = grammar.inherits.as_ref().map(|parent| {
        quote! { use super::#parent::*; }
    });

    let rules = grammar.rules.iter()
        .map(|r| rule::generate_rule(r, &custom_keywords))
        .collect::<Result<Vec<_>>>()?;

    Ok(quote! {
        pub mod #grammar_name {
            #![allow(unused_imports, unused_variables, dead_code, unused_braces, unused_parens)]
            
            pub const GRAMMAR_NAME: &str = stringify!(#grammar_name);

            use syn::parse::{Parse, ParseStream};
            use syn::Result;
            use syn::Token;
            use syn::ext::IdentExt; 
            
            pub mod rt {
                use syn::parse::ParseStream;
                use syn::Result;
                use syn::parse::discouraged::Speculative;
                use syn::ext::IdentExt; 
                use std::cell::{Cell, RefCell};

                thread_local! {
                    static IS_FATAL: Cell<bool> = const { Cell::new(false) };
                    static BEST_ERROR: RefCell<Option<syn::Error>> = const { RefCell::new(None) };
                }

                pub fn set_fatal(fatal: bool) {
                    IS_FATAL.set(fatal);
                }

                pub fn check_fatal() -> bool {
                    IS_FATAL.get()
                }

                fn record_error(err: syn::Error, start_span_debug: String) {
                    BEST_ERROR.with(|cell| {
                        let mut borrow = cell.borrow_mut();
                        
                        // Heuristic: Compare the error location to the start of the attempt.
                        // If they differ, we made progress (Deep Error).
                        // We prioritize Deep Errors over Shallow Errors.
                        let err_span_debug = format!("{:?}", err.span());
                        let is_deep = err_span_debug != start_span_debug;

                        match &*borrow {
                            None => {
                                *borrow = Some(err);
                            }
                            Some(_existing) => {
                                // If the new error is Deep, we prefer it.
                                // A more sophisticated check might compare actual line/column if available,
                                // but checking inequality with start is a good proxy for "moved forward".
                                if is_deep {
                                    *borrow = Some(err);
                                }
                            }
                        }
                    });
                }

                pub fn take_best_error() -> Option<syn::Error> {
                    BEST_ERROR.with(|cell| cell.borrow_mut().take())
                }

                pub fn attempt<T>(input: ParseStream, parser: impl FnOnce(ParseStream) -> Result<T>) -> Result<Option<T>> {
                    let was_fatal = check_fatal();
                    set_fatal(false);

                    let start_span = format!("{:?}", input.span());

                    let fork = input.fork();
                    let res = parser(&fork);
                    
                    let is_now_fatal = check_fatal();

                    match res {
                        Ok(res) => {
                            input.advance_to(&fork);
                            set_fatal(was_fatal);
                            Ok(Some(res))
                        }
                        Err(e) => {
                            if is_now_fatal {
                                set_fatal(true);
                                Err(e)
                            } else {
                                set_fatal(was_fatal);
                                record_error(e, start_span);
                                Ok(None)
                            }
                        }
                    }
                }

                pub fn attempt_recover<T>(input: ParseStream, parser: impl FnOnce(ParseStream) -> Result<T>) -> Result<Option<T>> {
                    let was_fatal = check_fatal();
                    set_fatal(false);

                    let fork = input.fork();
                    let res = parser(&fork);
                    
                    // For recovery, we don't care if it was fatal. We just want to know if it failed.
                    // We always backtrack (discard fork) on error.
                    // And we restore the previous fatal state.

                    match res {
                        Ok(res) => {
                            input.advance_to(&fork);
                            set_fatal(was_fatal);
                            Ok(Some(res))
                        }
                        Err(_) => {
                            set_fatal(was_fatal);
                            Ok(None)
                        }
                    }
                }

                pub fn parse_ident(input: ParseStream) -> Result<syn::Ident> {
                    input.call(syn::Ident::parse_any)
                }

                pub fn parse_int<T: std::str::FromStr>(input: ParseStream) -> Result<T> 
                where T::Err: std::fmt::Display {
                    input.parse::<syn::LitInt>()?.base10_parse()
                }

                pub fn skip_until(input: ParseStream, predicate: impl Fn(ParseStream) -> bool) -> Result<()> {
                    while !input.is_empty() && !predicate(input) {
                        if input.parse::<proc_macro2::TokenTree>().is_err() {
                            break; 
                        }
                    }
                    Ok(())
                }
            }

            #kw_defs
            #inheritance
            
            #(#rules)*
        }
    })
}
