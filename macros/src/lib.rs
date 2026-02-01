extern crate proc_macro;

use proc_macro::TokenStream;
use syn::parse_macro_input;
use quote::quote;
use syn_grammar_model::{parser, model, validator};

// Include modules
mod codegen;

const SYN_BUILTINS: &[&str] = &[
    "ident", "integer", "string", "rust_type", "rust_block", "lit_str",
    "lit_int", "lit_char", "lit_bool", "lit_float",
    "spanned_int_lit", "spanned_string_lit",
    "spanned_float_lit", "spanned_bool_lit", "spanned_char_lit"
];

/// The main macro for defining grammars.
///
/// See the [crate-level documentation](https://docs.rs/syn-grammar) for full syntax and usage details.
///
/// # Example
///
/// ```rust
/// use syn_grammar::grammar;
///
/// grammar! {
///     grammar MyGrammar {
///         rule main -> i32 = "42" -> { 42 }
///     }
/// }
/// ```
#[proc_macro]
pub fn grammar(input: TokenStream) -> TokenStream {
    // 1. Parsing: From TokenStream to syntactic AST (parser.rs)
    // parse_macro_input! automatically handles syntax errors and emits them as compile errors.
    let p_ast = parse_macro_input!(input as parser::GrammarDefinition);

    // 2. Transformation: From syntactic AST to semantic model (model.rs)
    // This uses your manual `impl From` implementation.
    let m_ast: model::GrammarDefinition = p_ast.into();

    // 3. Validation: Check for semantic errors (undefined rules, arg mismatch)
    if let Err(e) = validator::validate(&m_ast, SYN_BUILTINS) {
        return e.to_compile_error().into();
    }

    // 4. Code Generation: From model to finished Rust code (codegen.rs)
    match codegen::generate_rust(m_ast) {
        Ok(stream) => stream.into(), // Successful code
        Err(e) => e.to_compile_error().into(), // Emit generation error as compiler error
    }
}

#[doc(hidden)]
#[proc_macro]
pub fn include_grammar(_input: TokenStream) -> TokenStream {
    quote! {
        compile_error!("External files are removed in v0.2.0. Please move your grammar inline into grammar! { ... }.");
    }.into()
}
