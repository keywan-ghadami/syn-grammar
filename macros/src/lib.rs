extern crate proc_macro;

use proc_macro::TokenStream;
use syn::parse_macro_input;

// Include modules
mod parser;
mod model;
mod codegen;

/// The main macro.
/// Usage:
/// grammar! {
///     grammar MyGrammar { ... }
/// }
#[proc_macro]
pub fn grammar(input: TokenStream) -> TokenStream {
    // 1. Parsing: From TokenStream to syntactic AST (parser.rs)
    // parse_macro_input! automatically handles syntax errors and emits them as compile errors.
    let p_ast = parse_macro_input!(input as parser::GrammarDefinition);

    // 2. Transformation: From syntactic AST to semantic model (model.rs)
    // This uses your manual `impl From` implementation.
    let m_ast: model::GrammarDefinition = p_ast.into();

    // 3. Code Generation: From model to finished Rust code (codegen.rs)
    match codegen::generate_rust(m_ast) {
        Ok(stream) => stream.into(), // Successful code
        Err(e) => e.to_compile_error().into(), // Emit generation error as compiler error
    }
}
