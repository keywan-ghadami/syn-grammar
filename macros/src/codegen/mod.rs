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
            use syn_grammar::rt; 

            #kw_defs
            #inheritance
            
            #(#rules)*
        }
    })
}
