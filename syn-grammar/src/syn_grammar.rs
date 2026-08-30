#![doc = include_str!("../README.md")]
#![doc = "\n\n"]
#![doc = include_str!("../SYNTAX.md")]

pub mod rt {
    pub use super::token_filter;
    pub use super::builtins; // FIX: Das hat gefehlt!
    pub use grammar_kit::*;
}

pub use grammar_kit::testing;

/// Die Werttypen, die die eingebauten Regeln liefern (`Identifier`, `StringLiteral`,
/// `SpannedValue`). Sie stammen aus dem gemeinsamen Modell und werden hier
/// re-exportiert, damit Nutzer dafür nicht `syn-grammar-model` einbinden müssen.
pub use syn_grammar_model::model::types;

pub trait SynTestExt<O> {
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
