extern crate proc_macro;

use proc_macro::TokenStream;
use syn::parse_macro_input;

// Module einbinden
mod parser;
mod model;
mod codegen;

/// Das Haupt-Makro.
/// Nutzung:
/// grammar! {
///     grammar MyGrammar { ... }
/// }
#[proc_macro]
pub fn grammar(input: TokenStream) -> TokenStream {
    // 1. Parsing: Vom TokenStream zum syntaktischen AST (parser.rs)
    // parse_macro_input! kümmert sich automatisch um Syntax-Fehler und gibt sie als Compile-Error aus.
    let p_ast = parse_macro_input!(input as parser::GrammarDefinition);

    // 2. Transformation: Vom syntaktischen AST zum semantischen Modell (model.rs)
    // Hier greift deine manuelle `impl From` Implementierung.
    let m_ast: model::GrammarDefinition = p_ast.into();

    // 3. Code-Generierung: Vom Modell zum fertigen Rust-Code (codegen.rs)
    match codegen::generate_rust(m_ast) {
        Ok(stream) => stream.into(), // Erfolgreicher Code
        Err(e) => e.to_compile_error().into(), // Generierungs-Fehler als Compiler-Error ausgeben
    }
}
