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

pub mod model;
pub mod parser;
pub mod validator;
pub mod analysis;
