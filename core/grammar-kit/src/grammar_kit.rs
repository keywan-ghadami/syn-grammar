#![doc = include_str!("../README.md")]
// The public API of this crate is the runtime of generated parsers -
// undocumented items in it are a bug, not a blemish.
#![warn(missing_docs)]

/// The building blocks the generated code consists of.
#[cfg(feature = "syn")]
pub mod combinators;
/// The state carried along during a parse run: rule stack, error
/// high-water mark, group depth, lexical mode.
#[cfg(feature = "syn")]
pub mod context;
/// The runtime's error type and its selection rules.
#[cfg(feature = "syn")]
pub mod error;

/// The stream-driven runtime: `Stream`, `step`, `parse_syn`, forks.
#[cfg(feature = "testing")]
pub mod stream;

pub mod testing;

pub use grammar_kit_macros::with_span;

/// Constructs a value from parsed data and the position where it was found.
///
/// Implemented by the derive macro [`with_span`].
pub trait WithSpan<ParsedData> {
    /// Builds `Self` from `parsed_data` and the byte range `span`.
    fn with_span(parsed_data: ParsedData, span: std::ops::Range<usize>) -> Self;
}

// Re-export the encapsulated modules flat for the code generator
#[cfg(feature = "syn")]
pub use combinators::*;
#[cfg(feature = "syn")]
pub use context::*;
#[cfg(feature = "syn")]
pub use error::*;

pub use stream::*;
