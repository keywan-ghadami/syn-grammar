//! Token filters for emulating character-level primitives in a token stream.
//!
//! They work on the `Cursor` like the rest of the generated code: they read a
//! token via the bridge and check it afterwards. Before the switch to cursor
//! parsing they still took a `ParseStream` and were therefore no longer callable
//! from the generated code at all.

use crate::rt::{take_single, ParseError, ParseResult};
use syn::buffer::Cursor;
use syn::spanned::Spanned;
use syn::{Ident, LitInt};

/// Reads a token and checks it; if the check fails, an error arises at the
/// position where the parser was.
fn filtered<'a, T, F>(cursor: Cursor<'a>, check: F, expected: &str) -> ParseResult<'a, T>
where
    T: crate::rt::SingleToken + Spanned,
    F: FnOnce(&T) -> bool,
{
    let (value, next) = take_single::<T>(cursor)?;
    if check(&value) {
        Ok((value, next))
    } else {
        Err(ParseError::new(value.span(), format!("expected {}", expected)).with_cursor(cursor))
    }
}

/// An identifier consisting exclusively of letters.
pub fn alpha(cursor: Cursor<'_>) -> ParseResult<'_, Ident> {
    filtered(
        cursor,
        |i: &Ident| i.to_string().chars().all(char::is_alphabetic),
        "an alphabetic identifier",
    )
}

/// An identifier made of letters and digits.
pub fn alphanumeric(cursor: Cursor<'_>) -> ParseResult<'_, Ident> {
    filtered(
        cursor,
        |i: &Ident| i.to_string().chars().all(char::is_alphanumeric),
        "an alphanumeric identifier",
    )
}

/// An integer literal made of pure decimal digits.
pub fn digit(cursor: Cursor<'_>) -> ParseResult<'_, LitInt> {
    filtered(
        cursor,
        |l: &LitInt| l.base10_digits().chars().all(|c| c.is_ascii_digit()),
        "a numeric literal",
    )
}

/// An integer literal whose digits are valid hexadecimal.
pub fn hex_digit(cursor: Cursor<'_>) -> ParseResult<'_, LitInt> {
    filtered(
        cursor,
        |l: &LitInt| l.base10_digits().chars().all(|c| c.is_ascii_hexdigit()),
        "a hex literal",
    )
}

/// An integer literal whose digits are valid octal.
pub fn oct_digit(cursor: Cursor<'_>) -> ParseResult<'_, LitInt> {
    filtered(
        cursor,
        |l: &LitInt| l.base10_digits().chars().all(|c| c.is_digit(8)),
        "an octal literal",
    )
}
