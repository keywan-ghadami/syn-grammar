mod analysis;
mod pattern;
mod rule;

use crate::model::*;
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
            #![allow(unused_imports, unused_variables, dead_code, unused_braces)]
            
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
                use std::cell::Cell;

                thread_local! {
                    static IS_FATAL: Cell<bool> = const { Cell::new(false) };
                }

                pub fn set_fatal(fatal: bool) {
                    IS_FATAL.set(fatal);
                }

                pub fn check_fatal() -> bool {
                    IS_FATAL.get()
                }

                pub fn attempt<T>(input: ParseStream, parser: impl FnOnce(ParseStream) -> Result<T>) -> Result<Option<T>> {
                    let was_fatal = check_fatal();
                    set_fatal(false);

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
                                Ok(None)
                            }
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
            }

            #kw_defs
            #inheritance
            
            #(#rules)*
        }
    })
}
