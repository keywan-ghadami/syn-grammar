#![doc = include_str!("../README.md")]

extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    Result, Token,
};
use syn_grammar_model::parser::GrammarDefinition;

// Include modules
mod backend;
mod codegen;
mod monomorphize;

use backend::SynBackend;
use syn_grammar_model::parse_grammar;

#[proc_macro]
pub fn grammar(input: TokenStream) -> TokenStream {
    let file = match syn::parse::<GrammarFile>(input) {
        Ok(f) => f,
        Err(e) => return e.to_compile_error().into(),
    };

    let def = &file.grammar;
    let grammar_name = &def.name;
    let rules_macro_name = quote::format_ident!("{}_rules", grammar_name);

    let rules = &def.rules;
    let rules_tokens = quote! { #(#rules)* };

    // We use a local alias for grammar_core to ensure we can match it as an identifier
    // in macro_rules! without dealing with paths.
    let mut current_chain = quote! {
        _grammar_core! { #def }
    };

    for include in file.includes.iter().rev() {
        let macro_name = &include.macro_name;
        let alias = &include.alias;
        current_chain = quote! {
            #macro_name! { #alias, { #current_chain } }
        };
    }

    quote! {
        #[macro_export]
        macro_rules! #rules_macro_name {
            ($alias:ident, { $next:ident! { $($inner:tt)* } $($trailing:tt)* }) => {
                $next! {
                    $($inner)*
                    ruleset {
                        #rules_tokens
                    } as $alias;
                    $($trailing)*
                }
            };
        }

        #[allow(unused_imports)]
        use syn_grammar::grammar_core as _grammar_core;
        #current_chain
    }
    .into()
}

#[proc_macro]
pub fn grammar_core(input: TokenStream) -> TokenStream {
    let mut m_ast = match parse_grammar::<SynBackend>(input.into()) {
        Ok(ast) => ast,
        Err(e) => return e.to_compile_error().into(),
    };

    let monomorphizer = monomorphize::Monomorphizer::new(m_ast.rules);
    m_ast.rules = monomorphizer.process();

    match codegen::generate_rust(m_ast) {
        Ok(stream) => stream.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

mod kw {
    syn::custom_keyword!(include);
}

struct Include {
    macro_name: syn::Ident,
    alias: syn::Ident,
}

impl Parse for Include {
    fn parse(input: ParseStream) -> Result<Self> {
        let _include_token: kw::include = input.parse()?;
        let macro_name: syn::Ident = input.parse()?;
        let _as_token: Token![as] = input.parse()?;
        let alias: syn::Ident = input.parse()?;
        Ok(Include { macro_name, alias })
    }
}

struct GrammarFile {
    includes: Vec<Include>,
    grammar: GrammarDefinition,
}

impl Parse for GrammarFile {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut includes = Vec::new();
        while input.peek(kw::include) {
            includes.push(input.parse()?);
            let _ = input.parse::<Token![;]>()?;
        }
        let grammar: GrammarDefinition = input.parse()?;
        Ok(GrammarFile { includes, grammar })
    }
}
