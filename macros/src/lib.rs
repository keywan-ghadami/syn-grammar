extern crate proc_macro;

use proc_macro::TokenStream;
use syn::{parse_macro_input, LitStr};
use std::path::Path;
use std::fs;
use quote::quote;

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

/// Liest eine Grammatik-Datei zur Compile-Zeit und generiert den Parser.
/// Nutzung:
/// include_grammar!("grammar.g");
#[proc_macro]
pub fn include_grammar(input: TokenStream) -> TokenStream {
    // 1. Pfad einlesen
    let file_path_lit = parse_macro_input!(input as LitStr);
    let file_path = file_path_lit.value();

    // 2. Pfad relativ zum Cargo Manifest auflösen
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let path = Path::new(&manifest_dir).join(&file_path);

    if !path.exists() {
        return syn::Error::new(file_path_lit.span(), format!("File not found: {:?}", path))
            .to_compile_error()
            .into();
    }

    // 3. Datei lesen
    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => return syn::Error::new(file_path_lit.span(), format!("Error reading file: {}", e))
            .to_compile_error()
            .into(),
    };

    // 4. Parsen (String -> AST)
    // syn::parse_str nutzt den Parse-Trait von GrammarDefinition
    let p_ast: parser::GrammarDefinition = match syn::parse_str(&content) {
        Ok(ast) => ast,
        Err(e) => return e.to_compile_error().into(),
    };

    // 5. Transformation (AST -> Model)
    let m_ast: model::GrammarDefinition = p_ast.into();

    // 6. Code-Generierung
    let generated_code = match codegen::generate_rust(m_ast) {
        Ok(stream) => stream,
        Err(e) => return e.to_compile_error().into(),
    };

    // 7. Re-Compile Trigger
    // Wir fügen ein include_bytes! hinzu, das nicht genutzt wird.
    // Das zwingt Cargo dazu, dieses Makro neu auszuführen, wenn sich die Datei ändert.
    let path_str = path.to_string_lossy();
    let rebuild_trigger = quote! {
        const _: &[u8] = include_bytes!(#path_str);
    };

    quote! {
        #rebuild_trigger
        #generated_code
    }.into()
}
