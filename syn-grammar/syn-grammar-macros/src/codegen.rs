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
    /// Die Regel, deren Rumpf gerade erzeugt wird - in Leerzeichen-Form.
    /// Manche Meldungen nennen sie (z.B. `not(..)`: "in rule `main`").
    pub current_rule: String,
}

pub fn generate_rust(grammar: GrammarDefinition) -> Result<TokenStream> {
    let grammar_name = &grammar.name;
    let custom_keywords = analysis::collect_custom_keywords(&grammar);

    let ctx = CodegenContext {
        grammar: &grammar,
        custom_keywords: &custom_keywords,
        current_rule: String::new(),
    };

    let kw_defs = (!custom_keywords.is_empty()).then(|| {
        let defs = custom_keywords.iter().map(|k| {
            let ident = format_ident!("{}", k);
            quote! {
                syn::custom_keyword!(#ident);
                impl std::fmt::Display for #ident {
                    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        f.write_str(stringify!(#ident))
                    }
                }
            }
        });
        quote! { pub mod kw { #(#defs)* } }
    });

    let uses = &grammar.uses;

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

    let rules_stream = quote! { #(#rules)* };
    let rules_str = rules_stream.to_string();

    Ok(quote! {
        #[allow(non_snake_case)]
        pub mod #grammar_name {
            #![allow(unused_imports, unused_variables, dead_code, unused_braces, unused_parens)]
            #![allow(clippy::all)]

            pub const GRAMMAR_NAME: &str = stringify!(#grammar_name);
            pub const GENERATED_SOURCE: &str = #rules_str;

            use super::*;
            use syn::buffer::Cursor; // DIE WICHTIGSTE ÄNDERUNG
            use syn::Result;
            use syn::Token;
            use syn::ext::IdentExt;
            use syn::spanned::Spanned;

            use syn_grammar::rt;
            #[allow(unused_imports)]
            use syn_grammar::builtins::*;

            #kw_defs
            #(#uses)*
            #(#imports)*

            #rules_stream
        }
    })
}
