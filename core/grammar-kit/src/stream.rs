//! The stream-driven part of the runtime (ADR 15, stage 3).
//!
//! Up to stage 2 the generated parser worked exclusively on
//! [`syn::buffer::Cursor`]. For real syn AST types there was thus no way to a
//! `ParseStream`, so per call the remaining stream had to be materialised and a
//! complete new `TokenBuffer` built from it - quadratic in the length of the
//! enclosing delimiter group.
//!
//! Here the body runs on a `ParseBuffer` instead, which is built exactly once.
//! A `syn::Type` thus costs `input.parse::<T>()`, i.e. O(length of the type)
//! instead of O(rest).
//!
//! The cursor primitives remain unchanged: [`step`] runs them inside a
//! `step` episode and advances the stream by exactly the result.

use crate::{ParseError, ParseResult, PRIO_STRUCTURAL};
use proc_macro2::{Delimiter, Span};
use syn::buffer::Cursor;
use syn::parse::discouraged::Speculative;
use syn::parse::ParseBuffer;

/// The input stream of a rule.
///
/// Deliberately `ParseBuffer<'a>` and **not** syn's alias
/// `ParseStream<'a> = &'a ParseBuffer<'a>`. The alias equates the lifetime of
/// the reference with that of the tokens; a rule would then be
/// `fn rule<'a>(input: &'a ParseBuffer<'a>)`, and an `input.fork()` lives only
/// until the end of the stack frame. `'a` would thereby be shortened to that
/// frame and a `ParseError<'a>` from a fork could no longer leave the call -
/// exactly the errors the selection needs.
///
/// With `&Stream<'a>` the reference lifetime stays free and `'a` depends solely
/// on the tokens. That is the prerequisite for the whole error selection.
pub type Stream<'a> = ParseBuffer<'a>;

/// Result of a parse step on the stream.
///
/// Counterpart of [`ParseResult`]: there the progress is the returned cursor,
/// here it is inside the stream itself.
pub type StreamResult<'a, T> = Result<T, ParseError<'a>>;

/// Runs a cursor primitive on the stream and advances it.
///
/// `ParseBuffer::step` is the only public way to set the position of a
/// stream, and it demands that the closure works for **every** lifetime `'c`.
/// A `ParseError<'c>` therefore cannot be passed out - it carries a
/// `Cursor<'c>`.
///
/// So the error is carried through the barrier without its cursor and
/// re-anchored at the entry position outside. That is not an approximation:
/// the primitives ([`crate::take_single`], the character filters) report their
/// error at the entry position anyway.
///
/// The detour would disappear with a public `StepCursor::advance_to`, which
/// syn's own source describes as safe. A request is prepared under
/// `docs/upstream/syn-stepcursor-advance-to.md` (ADR 15, stage 4); until it is
/// through, the rule is: **a primitive may only fail at its entry position
/// here**, otherwise its error position shifts.
pub fn step<'a, T, F>(input: &Stream<'a>, f: F) -> StreamResult<'a, T>
where
    F: for<'c> FnOnce(Cursor<'c>) -> ParseResult<'c, T>,
{
    let here = input.cursor();
    let mut saved: Option<(Span, String, u8, bool)> = None;
    let result = input.step(|sc| match f(*sc) {
        Ok((value, after)) => Ok((value, after)),
        Err(e) => {
            let span = e.span;
            saved = Some((span, e.message, e.priority, e.is_fatal));
            // The text is taken from `saved` outside; this error only serves
            // to make `step` refuse to advance.
            Err(syn::Error::new(span, "unreachable"))
        }
    });

    match result {
        Ok(value) => Ok(value),
        Err(syn_error) => Err(match saved {
            Some((span, message, priority, fatal)) => {
                let mut e = ParseError::new(span, message)
                    .with_cursor(here)
                    .with_priority(priority);
                e.is_fatal = fatal;
                e
            }
            // Can only occur if `step` itself fails, not the closure.
            // Defensive, not the normal case.
            None => ParseError::new(syn_error.span(), syn_error.to_string()).with_cursor(here),
        }),
    }
}

/// Reads a syn AST type from the stream.
///
/// **The core of stage 3.** Previously this went through `invoke_syn_parser`,
/// which materialised the remaining stream and had `Parser::parse2` build a new
/// `TokenBuffer` from it - O(rest) per call. Now it is an ordinary `parse` call
/// on the existing buffer, i.e. O(length of the type).
///
/// The bound is [`crate::SynParsable`] instead of `Parse`, so that a `syn::`
/// type without `Parse` produces an understandable message instead of a raw
/// trait-bound error on generated code.
///
/// **After an error the stream may have advanced.** Unlike with the cursor,
/// resetting is not free here; the caller must work on a fork if it wants to
/// be able to reset. The code generator does that at every reset point.
pub fn parse_syn<'a, T: crate::SynParsable>(input: &Stream<'a>) -> StreamResult<'a, T> {
    let here = input.cursor();
    input.parse::<T>().map_err(|e| error_from_syn(e, here))
}

/// Like [`parse_syn`], but with a special parser - for types without `impl Parse`
/// (`syn::Attribute`, `syn::Pat`) or with a different entry point
/// (`Block::parse_within`, `Ident::parse_any`).
pub fn parse_with<'a, T, F>(input: &Stream<'a>, parser: F) -> StreamResult<'a, T>
where
    F: FnOnce(syn::parse::ParseStream) -> syn::Result<T>,
{
    let here = input.cursor();
    parser(input).map_err(|e| error_from_syn(e, here))
}

/// Adopts a `syn::Error`.
///
/// Span from syn - it is more precise for display. Progress from the entry
/// position. At the end of the input or of the group syn's error carries only
/// `Span::call_site()`; there the cursor is the better source.
fn error_from_syn<'a>(e: syn::Error, here: Cursor<'a>) -> ParseError<'a> {
    let span = if here.eof() { here.span() } else { e.span() };
    ParseError::new(span, e.to_string()).with_cursor(here)
}

/// A fork of the stream - the reset point.
///
/// As long as the fork is not applied via [`advance_to`], the stream stays
/// put. This replaces the cursor copy of the old design; it costs a small
/// `Rc` allocation, but the `TokenBuffer` build disappears.
pub fn fork<'a>(input: &Stream<'a>) -> Stream<'a> {
    input.fork()
}

/// Applies a successful fork to the stream. According to syn O(1), regardless
/// of the distance.
pub fn advance_to<'a>(input: &Stream<'a>, fork: &Stream<'a>) {
    input.advance_to(fork);
}

/// Descends into a delimiter group.
///
/// Returns the span of the delimiters and the stream of their content; the
/// outer stream is positioned after the group afterwards.
///
/// Uses `syn::__private::parse_{parens,braces,brackets}` - the functions
/// behind the macros `parenthesized!`/`braced!`/`bracketed!`. They are
/// `#[doc(hidden)]`, i.e. without a semver promise, and therefore encapsulated
/// here at exactly one place (like `Token::peek` in [`crate::peek_syn`]).
///
/// The macros themselves do not work: their error path is a bare
/// `return Err(syn::Error)`, which presupposes a function that returns exactly
/// that error type. `AnyDelimiter::parse_any_delimiter` does not work either -
/// its return value is shortened to the lifetime of `&self`, so no error from
/// the group could be carried outward any more.
pub fn group<'a>(
    input: &Stream<'a>,
    delimiter: Delimiter,
) -> StreamResult<'a, (proc_macro2::extra::DelimSpan, Stream<'a>)> {
    let here = input.cursor();
    let result = match delimiter {
        Delimiter::Parenthesis => {
            syn::__private::parse_parens(input).map(|g| (g.token.span, g.content))
        }
        Delimiter::Brace => syn::__private::parse_braces(input).map(|g| (g.token.span, g.content)),
        Delimiter::Bracket => {
            syn::__private::parse_brackets(input).map(|g| (g.token.span, g.content))
        }
        Delimiter::None => Err(syn::Error::new(here.span(), "expected delimited group")),
    };
    result.map_err(|_| {
        ParseError::at_cursor(here, "expected delimited group").with_priority(PRIO_STRUCTURAL)
    })
}

/// Takes exactly one token tree from the stream.
///
/// For the places that run over raw tokens: `until(..)` collects them,
/// `recover(..)` skips them up to the synchronisation marker.
pub fn take_token<'a>(input: &Stream<'a>) -> StreamResult<'a, proc_macro2::TokenTree> {
    step(input, |cursor| match cursor.token_tree() {
        Some((tt, after)) => Ok((tt, after)),
        None => Err(ParseError::at_cursor(cursor, "unexpected end of input")),
    })
}
