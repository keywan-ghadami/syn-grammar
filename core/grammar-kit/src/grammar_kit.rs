#![doc = include_str!("../README.md")]

// Die oeffentliche API dieses Crates ist die Laufzeit generierter Parser -
// undokumentierte Elemente darin sind ein Fehler, kein Schoenheitsmakel.
#![warn(missing_docs)]

/// Der Fehlertyp der Laufzeit und seine Auswahlregeln.
#[cfg(feature = "syn")]
pub mod error;
/// Der waehrend eines Parselaufs mitgefuehrte Zustand: Regelstapel,
/// Hochwasserstand der Fehler, Gruppentiefe, lexikalischer Modus.
#[cfg(feature = "syn")]
pub mod context;
/// Die Bausteine, aus denen der generierte Code besteht.
#[cfg(feature = "syn")]
pub mod combinators;

/// Fluente Zusicherungen zum Testen generierter Parser.
#[cfg(feature = "testing")]
pub mod testing;

pub use grammar_kit_macros::with_span;

/// Konstruiert einen Wert aus geparsten Daten und der Stelle, an der sie standen.
///
/// Wird vom Ableitungsmakro [`with_span`] implementiert.
pub trait WithSpan<ParsedData> {
    /// Baut `Self` aus `parsed_data` und dem Byte-Bereich `span`.
    fn with_span(parsed_data: ParsedData, span: std::ops::Range<usize>) -> Self;
}

// Exportiere die gekapselten Module flach für den Codegenerator
#[cfg(feature = "syn")]
pub use error::*;
#[cfg(feature = "syn")]
pub use context::*;
#[cfg(feature = "syn")]
pub use combinators::*;
