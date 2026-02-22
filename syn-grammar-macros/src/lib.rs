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
    // Generate unique alias for grammar_core to avoid conflicts when multiple grammars are defined in the same scope
    let core_alias = quote::format_ident!("_grammar_core_{}", grammar_name);

    let rules = &def.rules;
    let rules_tokens = quote! { #(#rules)* };

    // 1. Build the chain for the grammar definition (the code that runs here)
    let mut current_chain = quote! {
        #core_alias! { #def }
    };

    for include in file.includes.iter().rev() {
        let macro_name = &include.macro_name;
        let alias = &include.alias;
        current_chain = quote! {
            #macro_name! { #alias, { #current_chain } }
        };
    }

    // Helper to generate the expansion body
    // we need a closure or function because we generate it twice
    let generate_expansion = |acc_tokens: proc_macro2::TokenStream| {
        if file.includes.is_empty() {
            quote! {
                $next! {
                    @accum (
                        #acc_tokens
                        ruleset {
                            #rules_tokens
                        } as $alias;
                    )
                    $($inner)*
                    $($rest)*
                }
            }
        } else {
            let first = &file.includes[0];
            let rest = &file.includes[1..];
            
            let mut inner_chain = quote! {
                $next! { 
                    $($inner)* 
                    $($rest)* 
                }
            };

            for include in rest.iter().rev() {
                 let m = &include.macro_name;
                 let a = &include.alias;
                 inner_chain = quote! {
                     #m! { #a, { #inner_chain } }
                 };
            }

            let first_macro = &first.macro_name;
            let first_alias = &first.alias;

            quote! {
                #first_macro! {
                    @accum (
                        #acc_tokens
                        ruleset {
                            #rules_tokens
                        } as $alias;
                    )
                    #first_alias, 
                    { 
                        #inner_chain 
                    }
                }
            }
        }
    };

    let expansion_accum = generate_expansion(quote! { $($acc)* });
    let expansion_entry = generate_expansion(quote! {});

    quote! {
        // Define the macro locally
        macro_rules! #rules_macro_name {
            // Recursive branch: receives accumulated rules in @accum
            (@accum ($($acc:tt)*) $alias:ident, { $next:ident! { $($inner:tt)* } } $($rest:tt)*) => {
                #expansion_accum
            };

            // Entry branch: called without @accum, initializes it
            ($alias:ident, { $next:ident! { $($inner:tt)* } } $($rest:tt)*) => {
                #expansion_entry
            };
        }
        
        // Export the macro so it can be used outside this module but within the crate
        #[allow(unused_imports)]
        pub(crate) use #rules_macro_name;

        #[allow(unused_imports)]
        use syn_grammar::grammar_core as #core_alias;
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
    macro_name: syn::Path,
    alias: syn::Ident,
}

impl Parse for Include {
    fn parse(input: ParseStream) -> Result<Self> {
        let _include_token: kw::include = input.parse()?;
        let macro_name: syn::Path = input.parse()?;
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
