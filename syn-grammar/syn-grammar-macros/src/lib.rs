#![doc = include_str!("../README.md")]

extern crate proc_macro;

use proc_macro::TokenStream;
use syn_grammar_model::parse_grammar;

// Include modules
mod backend;
mod codegen;
mod monomorphize;

use backend::SynBackend;

#[proc_macro]
pub fn grammar(input: TokenStream) -> TokenStream {
    // The new `grammar!` macro directly delegates to `grammar_core` logic.
    // It no longer handles complex macro expansion for composition.
    grammar_core(input)
}

fn grammar_core(input: TokenStream) -> TokenStream {
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
