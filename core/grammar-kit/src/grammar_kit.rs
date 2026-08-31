#![doc = include_str!("../README.md")]

#[cfg(feature = "syn")]
pub mod error;
#[cfg(feature = "syn")]
pub mod context;
#[cfg(feature = "syn")]
pub mod combinators;

#[cfg(feature = "testing")]
pub mod testing;

pub use grammar_kit_macros::with_span;

pub trait WithSpan<ParsedData> {
    fn with_span(parsed_data: ParsedData, span: std::ops::Range<usize>) -> Self;
}

// Exportiere die gekapselten Module flach für den Codegenerator
#[cfg(feature = "syn")]
pub use error::*;
#[cfg(feature = "syn")]
pub use context::*;
#[cfg(feature = "syn")]
pub use combinators::*;
