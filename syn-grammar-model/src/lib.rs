//! # syn-grammar-model
//!
//! This library contains the shared logic for parsing, validating, and analyzing
//! `syn-grammar` definitions. It is intended to be used by procedural macros
//! that generate parsers or documentation from the grammar DSL.
//!
//! ## Pipeline
//!
//! 1. **[parser]**: Parse input tokens into a syntactic AST.
//! 2. **[model]**: Convert the AST into a semantic model (via `Into`).
//! 3. **[validator]**: Validate the model for semantic correctness.
//! 4. **[analysis]**: Extract information (keywords, recursion) for code generation.

use proc_macro2::TokenStream;
use syn::Result;

pub mod analysis;
pub mod model;
pub mod parser;
pub mod validator;

/// Primitives that are conceptually portable across different backends (token or character based).
/// A grammar using only these should be portable.
pub const PORTABLE_BUILTINS: &[&str] = &[
    // High-level conceptual tokens
    "ident",
    "integer",
    "string",
    "float",
    // Low-level character classes
    "eof",
    "whitespace",
    "alpha",
    "digit",
    "alphanumeric",
    "hex_digit",
    "oct_digit",
    "any_byte",
];

/// Primitives that are intrinsically tied to the `syn` crate and its AST.
/// A grammar using these is not portable to a non-syn backend.
pub const SYN_SPECIFIC_BUILTINS: &[&str] = &[
    "rust_type",
    "rust_block",
    "lit_str",
    "lit_int",
    "lit_char",
    "lit_bool",
    "lit_float",
    "outer_attrs",
    // Deprecated spanned variants
    "spanned_int_lit",
    "spanned_string_lit",
    "spanned_float_lit",
    "spanned_bool_lit",
    "spanned_char_lit",
];

/// Reusable pipeline: Parses, transforms, and validates the grammar.
///
/// This encapsulates the standard 3-step process used by all backends.
///
/// This function uses the default built-ins for `syn-grammar`.
/// If you are building a custom backend (e.g. `winnow-grammar`), use `parse_grammar_with_builtins` instead.
pub fn parse_grammar(input: TokenStream) -> Result<model::GrammarDefinition> {
    let all_builtins: Vec<&str> = PORTABLE_BUILTINS
        .iter()
        .cloned()
        .chain(SYN_SPECIFIC_BUILTINS.iter().cloned())
        .collect();
    parse_grammar_with_builtins(input, &all_builtins)
}

/// Reusable pipeline with custom built-ins.
///
/// Use this if your backend supports a different set of built-in rules.
pub fn parse_grammar_with_builtins(
    input: TokenStream,
    valid_builtins: &[&str],
) -> Result<model::GrammarDefinition> {
    // 1. Parsing: From TokenStream to syntactic AST
    let p_ast: parser::GrammarDefinition = syn::parse2(input)?;

    // 2. Transformation: From syntactic AST to semantic model
    let m_ast: model::GrammarDefinition = p_ast.into();

    // 3. Validation: Check for semantic errors
    validator::validate(&m_ast, valid_builtins)?;

    Ok(m_ast)
}
