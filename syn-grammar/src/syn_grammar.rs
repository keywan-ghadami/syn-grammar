#![doc = include_str!("../README.md")]
#![doc = "\n\n"]
#![doc = include_str!("../SYNTAX.md")]
// User-facing API: undocumented public items are a bug.
#![warn(missing_docs)]

/// Runtime environment for the generated code.
///
/// The code generator writes exclusively `rt::…` paths, so that a grammar
/// makes no assumptions about the module structure of the user's crate.
/// Not intended as public API.
pub mod rt {
    pub use super::builtins;
    pub use super::token_filter;
    pub use grammar_kit::*;
}

pub use grammar_kit::testing;

/// The value types the built-in rules return (`Identifier`, `StringLiteral`,
/// `SpannedValue`). They come from the shared model and are re-exported here
/// so that users do not need to depend on `syn-grammar-model` for them.
pub use syn_grammar_model::model::types;

/// Shorthand for tests: apply a generated parser function directly to a
/// `&str` and get the result as a [`testing::TestResult`].
pub trait SynTestExt<O> {
    /// Parses `input` and wraps success and failure alike in a `TestResult`,
    /// including the source text for the failure output.
    fn parse_test(self, input: &str) -> testing::TestResult<O, syn::Error>;
}

impl<F, O> SynTestExt<O> for F
where
    F: FnOnce(syn::parse::ParseStream) -> syn::Result<O>,
    O: std::fmt::Debug,
{
    fn parse_test(self, input: &str) -> testing::TestResult<O, syn::Error> {
        let parser = |input: syn::parse::ParseStream| self(input);
        match syn::parse::Parser::parse_str(parser, input) {
            Ok(val) => testing::TestResult::new(Ok(val)).with_source(input),
            Err(e) => testing::TestResult::new(Err(e)).with_source(input),
        }
    }
}

pub use syn_grammar_macros::grammar;

#[doc(hidden)]
pub mod builtins;
pub mod token_filter;
