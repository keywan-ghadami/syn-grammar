#![doc = include_str!("../README.md")]
#![doc = "\n\n"]
#![doc = include_str!("../SYNTAX.md")]

// 1. Export runtime modules
pub mod rt {
    pub use super::token_filter;
    pub use grammar_kit::*;
}

pub use grammar_kit::testing;

/// Extension trait: Erlaubt `.parse_str("...")` auf unseren generierten Cursor-basierten Parsern.
pub trait ParserExt<O> {
    fn parse_str(self, input: &str) -> Result<O, grammar_kit::error::ParseError>;
}

impl<F, O> ParserExt<O> for F
where
    // Das ist die Signatur, die unser Codegenerator erzeugt!
    F: for<'a> FnOnce(syn::buffer::Cursor<'a>) -> grammar_kit::error::ParseResult<'a, O>,
{
    fn parse_str(self, input: &str) -> Result<O, grammar_kit::error::ParseError> {
        // 1. String zu TokenStream
        let token_stream = match input.parse::<proc_macro2::TokenStream>() {
            Ok(ts) => ts,
            Err(e) => return Err(grammar_kit::error::ParseError::new(e.span(), e.to_string())),
        };
        
        // 2. TokenStream zu Buffer & Cursor
        let buffer = syn::buffer::TokenBuffer::new2(token_stream);
        let cursor = buffer.begin();

        // 3. Parsen und EOF prüfen
        match self(cursor) {
            Ok((val, remaining)) => {
                if !remaining.eof() {
                    Err(grammar_kit::error::ParseError::new(remaining.span(), "unexpected token"))
                } else {
                    Ok(val)
                }
            }
            Err(e) => Err(e),
        }
    }
}

/// Extension trait für direkte .parse_test() Aufrufe.
pub trait SynTestExt<O> {
    fn parse_test(self, input: &str) -> testing::TestResult<O, grammar_kit::error::ParseError>;
}

impl<F, O> SynTestExt<O> for F
where
    F: for<'a> FnOnce(syn::buffer::Cursor<'a>) -> grammar_kit::error::ParseResult<'a, O>,
    O: std::fmt::Debug,
{
    fn parse_test(self, input: &str) -> testing::TestResult<O, grammar_kit::error::ParseError> {
        use testing::Testable;
        self.parse_str(input).test().with_source(input)
    }
}

pub use syn_grammar_macros::grammar;

#[doc(hidden)]
pub mod builtins;
pub mod token_filter;
