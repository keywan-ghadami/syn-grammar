mod pattern;
mod rule;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::collections::HashSet;
use syn::Result;
use syn_grammar_model::{analysis, model::*};

pub struct CodegenContext<'a> {
    pub grammar: &'a GrammarDefinition,
    pub custom_keywords: &'a HashSet<String>,
}

pub fn generate_rust(grammar: GrammarDefinition) -> Result<TokenStream> {
    let grammar_name = &grammar.name;
    let custom_keywords = analysis::collect_custom_keywords(&grammar);

    let ctx = CodegenContext {
        grammar: &grammar,
        custom_keywords: &custom_keywords,
    };

    let kw_defs = (!custom_keywords.is_empty()).then(|| {
        let defs = custom_keywords.iter().map(|k| {
            let ident = format_ident!("{}", k);
            quote! { syn::custom_keyword!(#ident); }
        });
        quote! { pub mod kw { #(#defs)* } }
    });

    let uses = &grammar.uses;

    // Generate imports inside the module
    let imports = grammar.imports.iter().map(|imp| {
        let path = &imp.path;
        let alias = &imp.alias;
        quote! { use #path as #alias; }
    });

    let rules = grammar
        .rules
        .iter()
        .map(|r| rule::generate_rule(r, &ctx))
        .collect::<Result<Vec<_>>>()?;

    // Capture the rules as a TokenStream to reuse for both code generation and string introspection
    let rules_stream = quote! { #(#rules)* };
    let rules_str = rules_stream.to_string();

    Ok(quote! {
        pub mod #grammar_name {
            #![allow(unused_imports, unused_variables, dead_code, unused_braces, unused_parens)]
            #![allow(clippy::all)]

            pub const GRAMMAR_NAME: &str = stringify!(#grammar_name);

            /// The generated source code of the rules, used for testing verification.
            pub const GENERATED_SOURCE: &str = #rules_str;

            use super::*;
            use syn::parse::{Parse, ParseStream};
            use syn::Result;
            use syn::Token;
            use syn::ext::IdentExt;
            use syn::spanned::Spanned;

            // Import runtime from syn_grammar
            use syn_grammar::rt;

            // Import builtins (can be shadowed by local imports or rules)
            #[allow(unused_imports)]
            use syn_grammar::builtins::*;

            #kw_defs

            #(#uses)*
            #(#imports)*

            #rules_stream
        }
    })
}
