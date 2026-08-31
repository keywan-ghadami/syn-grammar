//! Token filters for emulating character-level primitives in a token stream.
//!
//! Sie arbeiten auf dem `Cursor` wie der uebrige generierte Code: sie lesen ein
//! Token ueber die Bruecke und pruefen es nach. Vor der Umstellung auf
//! Cursor-Parsing nahmen sie noch einen `ParseStream` und waren dadurch aus dem
//! generierten Code gar nicht mehr aufrufbar.

use crate::rt::{invoke_syn_parser, ParseError, ParseResult};
use syn::buffer::Cursor;
use syn::spanned::Spanned;
use syn::{Ident, LitInt};

/// Liest ein Token und prueft es; schlaegt die Pruefung fehl, entsteht ein Fehler
/// an der Stelle, an der der Parser stand.
fn filtered<'a, T, F>(cursor: Cursor<'a>, pruefung: F, erwartet: &str) -> ParseResult<'a, T>
where
    T: syn::parse::Parse + Spanned,
    F: FnOnce(&T) -> bool,
{
    let (wert, next) = invoke_syn_parser::<T>(cursor)?;
    if pruefung(&wert) {
        Ok((wert, next))
    } else {
        Err(ParseError::new(wert.span(), format!("expected {}", erwartet)).with_cursor(cursor))
    }
}

/// Ein Bezeichner, der ausschliesslich aus Buchstaben besteht.
pub fn alpha(cursor: Cursor<'_>) -> ParseResult<'_, Ident> {
    filtered(
        cursor,
        |i: &Ident| i.to_string().chars().all(char::is_alphabetic),
        "an alphabetic identifier",
    )
}

/// Ein Bezeichner aus Buchstaben und Ziffern.
pub fn alphanumeric(cursor: Cursor<'_>) -> ParseResult<'_, Ident> {
    filtered(
        cursor,
        |i: &Ident| i.to_string().chars().all(char::is_alphanumeric),
        "an alphanumeric identifier",
    )
}

/// Ein Ganzzahlliteral aus reinen Dezimalziffern.
pub fn digit(cursor: Cursor<'_>) -> ParseResult<'_, LitInt> {
    filtered(
        cursor,
        |l: &LitInt| l.base10_digits().chars().all(|c| c.is_ascii_digit()),
        "a numeric literal",
    )
}

/// Ein Ganzzahlliteral, dessen Ziffern hexadezimal gueltig sind.
pub fn hex_digit(cursor: Cursor<'_>) -> ParseResult<'_, LitInt> {
    filtered(
        cursor,
        |l: &LitInt| l.base10_digits().chars().all(|c| c.is_ascii_hexdigit()),
        "a hex literal",
    )
}

/// Ein Ganzzahlliteral, dessen Ziffern oktal gueltig sind.
pub fn oct_digit(cursor: Cursor<'_>) -> ParseResult<'_, LitInt> {
    filtered(
        cursor,
        |l: &LitInt| l.base10_digits().chars().all(|c| c.is_digit(8)),
        "an octal literal",
    )
}
