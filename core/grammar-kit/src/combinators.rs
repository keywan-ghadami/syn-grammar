use crate::{
    gabel, uebernehmen, ParseContext, ParseError, ParseResult, StreamResult, Strom,
    PRIO_AGGREGATED, PRIO_LABELED, PRIO_STRUCTURAL,
};
use syn::buffer::Cursor;

/// Allows peeking for specific syn tokens on a cursor.
pub fn peek_syn<P: syn::parse::Peek>(cursor: Cursor<'_>, token: P) -> bool {
    // Pure pointer arithmetic, no allocation - exactly what syn's own
    // `ParseStream::peek` does (`parse.rs`: `T::Token::peek(self.cursor())`).
    //
    // Previously a token window was materialised here and a complete
    // `TokenBuffer` built from it. That was wrong twice over: building the
    // buffer costs more than the peek itself, and a single token can be an
    // arbitrarily large delimiter group - `cursor.token_tree()` returns
    // `{ ...1000 tokens... }` as ONE tree. So the "small window" was small in
    // name only.
    //
    // `Peek::Token` and `Token::peek` are `#[doc(hidden)]` but publicly
    // reachable. No semver promise - hence encapsulated at exactly this one
    // place.
    let _ = token;
    <P::Token as syn::token::Token>::peek(cursor)
}

/// Marker for types that may appear directly as `syn::Foo` in a grammar.
///
/// Semantically identical to `syn::parse::Parse` - its only purpose is the
/// error message. The code generator (`codegen/pattern.rs`) lets every path
/// through whose first segment is `syn`, without being able to check whether
/// the type is parseable at all. Without this marker the user got a raw
/// trait-bound error for `syn::Field` or `syn::Attribute` that pointed at
/// generated code they never wrote.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be used directly in a grammar",
    note = "A `syn::` type is usable in a grammar only if it implements `syn::parse::Parse`.",
    note = "Types such as `syn::Field`, `syn::Attribute` or `syn::Pat` do not - for them there are built-in rules (`named_field`, `outer_attrs`/`inner_attrs`, `pat`).",
    note = "For everything else: an `extern` rule with its own parser function."
)]
pub trait SynParsable: syn::parse::Parse {}

impl<T: syn::parse::Parse> SynParsable for T {}

/// Finishes a chain of alternatives and picks the message that goes outward.
///
/// `best` is the best individual error from the branches, `expected` collects the labels
/// of the branches that already failed at their boundary (i.e. without consuming a single
/// token). Implements ADR 13, points 6 and 7.
pub fn finish_variants<'a>(
    best: Option<ParseError<'a>>,
    mut expected: Vec<String>,
    start: Cursor<'a>,
    fallback_msg: &str,
) -> ParseError<'a> {
    // An error that got past the start point is more meaningful than the list of
    // alternatives at the start position - then `expected one of:` must not appear
    // at all (ADR 13, point 7).
    if let Some(b) = &best {
        let kam_weiter = b.at.map(|at| at > start).unwrap_or(false);
        if kam_weiter || b.priority >= PRIO_STRUCTURAL {
            return best.unwrap();
        }
    }

    expected.sort();
    expected.dedup();

    // What is actually at that position? (ADR 13, point 3)
    let gefunden = match start.token_tree() {
        Some((tt, _)) => {
            let t = tt.to_string();
            if t.trim().is_empty() {
                String::new()
            } else {
                format!("; found unexpected token `{}`", t)
            }
        }
        None => String::new(),
    };

    match expected.len() {
        0 => best.unwrap_or_else(|| ParseError::at_cursor(start, fallback_msg)),
        1 => ParseError::at_cursor(start, format!("expected `{}`{}", expected[0], gefunden))
            .with_priority(PRIO_LABELED),
        _ => {
            let liste = expected
                .iter()
                .map(|e| format!("`{}`", e))
                .collect::<Vec<_>>()
                .join(", ");
            ParseError::at_cursor(start, format!("expected one of: {}{}", liste, gefunden))
                .with_priority(PRIO_AGGREGATED)
        }
    }
}

/// Labels a failed attempt at a list item.
///
/// If the item failed right at its start position, its internal message says nothing
/// about the list - then the item's expectation takes its place, if necessary with the
/// note that the input or the group has ended (ADR 13, point 3). If it made progress,
/// however, its own error is the more meaningful message and stays untouched.
///
/// In both cases the error's rule stack is preserved - the item index (`in item 3`) is
/// already there.
fn label_missing_item<'a>(
    mut e: ParseError<'a>,
    at: Cursor<'a>,
    item_name: &str,
    ctx: &ParseContext<'a>,
    prio: u8,
) -> ParseError<'a> {
    if ersetzt_meldung(&e, at) {
        e.message = erwartung_item(at, item_name, ctx);
        e.span = at.span();
    }
    e.priority = e.priority.max(prio);
    e
}

/// Does the item's expectation replace its internal message?
///
/// Only if the item did not make any progress at all - otherwise its own message
/// is the more meaningful one (ADR 13, point 6).
///
/// And not even then if the error already carries a label of its own:
/// `finish_variants` turns that into `expected `x`; found unexpected token `y``,
/// which additionally names what was actually there. That message is richer than
/// `expected x` and stays.
///
/// The exception is the end of the input or of the group: there the note that the
/// scope ends matters more than any enumeration - otherwise the message claims
/// something could have been there where nothing follows any more (ADR 13, point 3).
fn ersetzt_meldung(e: &ParseError<'_>, at: Cursor<'_>) -> bool {
    e.at == Some(at) && (e.priority < PRIO_LABELED || at.eof())
}

/// The expectation that applies at the position of a missing list item.
///
/// At the end of the input or of the group this is said explicitly - "expected function
/// argument" alone would be misleading there (ADR 13, point 3).
fn erwartung_item(at: Cursor<'_>, item_name: &str, ctx: &ParseContext<'_>) -> String {
    if at.eof() {
        format!("{}, expected {}", ctx.end_of_scope_msg(), item_name)
    } else {
        format!("expected {}", item_name)
    }
}

/// A list of `item_parser`, separated by `sep_parser`.
///
/// `min` is the minimum count, `trailing` allows a dangling separator.
/// `item_name` names the items in error messages and ends up as
/// `"<item_name> <index>"` on the live rule stack - hence
/// `in function parameter 2`.
///
/// Every attempt runs on a [`gabel`] (fork); only success is applied via
/// [`uebernehmen`] (advance_to). In the cursor design, resetting was free
/// (simply do not use the new cursor); on the stream it costs the fork
/// - in exchange the `TokenBuffer` build per AST type disappears (ADR 15, stage 3).
pub fn parse_separated<'a, T, P, S>(
    input: &Strom<'a>,
    ctx: &mut ParseContext<'a>,
    mut item_parser: P,
    mut sep_parser: S,
    min: usize,
    trailing: bool,
    item_name: &str,
) -> StreamResult<'a, Vec<T>>
where
    P: FnMut(&Strom<'a>, &mut ParseContext<'a>) -> StreamResult<'a, T>,
    S: FnMut(&Strom<'a>, &mut ParseContext<'a>) -> StreamResult<'a, ()>,
{
    let mut items = Vec::new();

    // First item. The item name is on the live stack during the attempt - only
    // that way does an error recorded DEEP inside the item carry the list index.
    let start = input.cursor();
    let erste_gabel = gabel(input);
    ctx.enter_rule(&format!("{} 1", item_name));
    let erstes = item_parser(&erste_gabel, ctx);
    ctx.exit_rule();
    match erstes {
        Ok(item) => {
            uebernehmen(input, &erste_gabel);
            items.push(item);
        }
        Err(mut e) => {
            if min > 0 {
                // If the message is replaced, the internal rule stack no longer
                // says anything about the error either - then only the list
                // context counts. If the message stays, so does the stack.
                if ersetzt_meldung(&e, start) {
                    e.rule_stack.clear();
                }
                // The error belongs to the first item of the list (ADR 13, point 11).
                e.push_rule(&format!("{} 1", item_name));
                return Err(label_missing_item(
                    e,
                    start,
                    item_name,
                    ctx,
                    PRIO_STRUCTURAL,
                ));
            }
            // An empty list is allowed - but the reason why no item came is
            // recorded. Otherwise only a generic message remains later.
            //
            // If the item did NOT make progress, its internal message says
            // nothing about the list; then "expected <item>" is the answer, and
            // it needs the rank of a label. Without that, a token error recorded
            // later at the same position wins the tie - for `fn f( 123 )` e.g.
            // the optional `","?`, which turned "expected function argument"
            // into a meaningless "expected `,`". See ADR 13, point 6.
            //
            // If it made progress, everything stays untouched: its own message
            // is then the more meaningful one, together with its rule stack.
            if ersetzt_meldung(&e, start) {
                e.rule_stack.clear();
            }
            let mut e = label_missing_item(e, start, item_name, ctx, PRIO_LABELED);
            e.push_rule(&format!("{} 1", item_name));
            ctx.record_failure(&e);
            return Ok(items);
        }
    }

    loop {
        let mut sep_ctx = ctx.clone();

        // Try the separator - on a fork, so that on failure the stream stays
        // BEFORE the separator.
        let sep_gabel = gabel(input);
        sep_ctx.enter_rule("separator");
        let sep_res = sep_parser(&sep_gabel, &mut sep_ctx);
        sep_ctx.exit_rule();
        match sep_res {
            Ok(()) => {
                let nach_sep = sep_gabel.cursor();
                let mut item_ctx = sep_ctx.clone();

                // Try the item AFTER the separator, again on its own fork.
                let item_gabel = gabel(&sep_gabel);
                item_ctx.enter_rule(&format!("{} {}", item_name, items.len() + 1));
                let item_res = item_parser(&item_gabel, &mut item_ctx);
                item_ctx.exit_rule();
                match item_res {
                    Ok(item) => {
                        uebernehmen(input, &item_gabel);
                        items.push(item);
                        *ctx = item_ctx;
                    }
                    Err(mut e) => {
                        // See above: if the message is replaced, the internal
                        // stack contributes nothing.
                        if ersetzt_meldung(&e, nach_sep) {
                            e.rule_stack.clear();
                        }
                        // Index of the ATTEMPTED item, 1-based.
                        e.push_rule(&format!("{} {}", item_name, items.len() + 1));
                        if trailing {
                            // A dangling separator is allowed: it BELONGS to the list
                            // and is consumed. Without this it stayed in the stream and
                            // the surrounding rule failed on it.
                            uebernehmen(input, &sep_gabel);
                            *ctx = sep_ctx;
                            ctx.record_failure(&e);
                            break;
                        } else {
                            // Soft reset instead of hard failure: the stream stays
                            // BEFORE the separator so that a following rule (such as
                            // a `","?`) can still process it. `paren(args:list? ","?)`
                            // grammars rely on exactly that.
                            //
                            // The reason is recorded - if nothing matches afterwards,
                            // it resurfaces instead of being replaced by a generic
                            // message. The REAL error is enriched, so that its rule
                            // stack and, if it was deeper, its position are preserved.
                            let markiert =
                                label_missing_item(e, nach_sep, item_name, ctx, PRIO_STRUCTURAL);
                            ctx.record_failure(&markiert);
                            ctx.absorb(&item_ctx);
                            break;
                        }
                    }
                }
            }
            Err(mut e) => {
                // No more separator - the list is done. Why it did not continue
                // here is recorded nonetheless (ADR 13, point 11).
                e.rule_stack.clear();
                e.push_rule("separator");
                ctx.record_failure(&e);
                ctx.absorb(&sep_ctx);
                break;
            }
        }
    }

    if items.len() < min {
        return Err(ParseError::at_cursor(
            input.cursor(),
            format!(
                "expected at least {} {}s, found {}",
                min,
                item_name,
                items.len()
            ),
        )
        .with_priority(PRIO_STRUCTURAL));
    }

    Ok(items)
}

/// Combinator for repetitions without a separator.
///
/// Counterpart of [`parse_separated`]. A structural error (priority
/// >= 50) aborts the loop hard instead of merely ending it.
pub fn parse_repeated<'a, T, P>(
    input: &Strom<'a>,
    ctx: &mut ParseContext<'a>,
    mut item_parser: P,
    min: usize,
    item_name: &str,
) -> StreamResult<'a, Vec<T>>
where
    P: FnMut(&Strom<'a>, &mut ParseContext<'a>) -> StreamResult<'a, T>,
{
    let mut items = Vec::new();

    loop {
        let vorher = input.cursor();
        let item_gabel = gabel(input);
        let mut item_ctx = ctx.clone();
        item_ctx.enter_rule(&format!("{} {}", item_name, items.len() + 1));
        let item_res = item_parser(&item_gabel, &mut item_ctx);
        item_ctx.exit_rule();
        match item_res {
            Ok(item) => {
                // No progress despite success -> otherwise an endless loop.
                if item_gabel.cursor() == vorher {
                    break;
                }
                uebernehmen(input, &item_gabel);
                items.push(item);
                *ctx = item_ctx;
            }
            Err(e) => {
                // Pass structural/fatal errors through; everything else ends
                // the repetition normally.
                if e.priority >= PRIO_STRUCTURAL {
                    return Err(e);
                }
                // The repetition ends normally - the reason is recorded.
                ctx.record_failure(&e);
                ctx.absorb(&item_ctx);
                break;
            }
        }
    }

    if items.len() < min {
        return Err(ParseError::at_cursor(
            input.cursor(),
            format!(
                "expected at least {} {}s, found {}",
                min,
                item_name,
                items.len()
            ),
        )
        .with_priority(PRIO_STRUCTURAL));
    }

    Ok(items)
}

/// A type that consists of exactly one token and can therefore be read directly
/// from the `Cursor` in O(1).
///
/// Even with [`crate::parse_syn`] (ADR 15, stage 3) this pays off: an
/// `input.parse::<T>()` goes through syn's expectation and error machinery,
/// whereas here a pointer comparison suffices. [`crate::schritt`] runs these
/// primitives on the stream.
///
/// The error messages are word-for-word identical to syn's - several tests
/// check them by substring.
pub trait SingleToken: Sized {
    /// Reads the token if it matches. `None` means: does not match.
    fn take(cursor: Cursor<'_>) -> Option<(Self, Cursor<'_>)>;
    /// The message when it does not match - word-for-word identical to syn.
    fn erwartet() -> &'static str;
}

/// Reads a [`SingleToken`] type from the cursor in O(1).
pub fn take_single<'a, T: SingleToken>(cursor: Cursor<'a>) -> ParseResult<'a, T> {
    match T::take(cursor) {
        Some((wert, next)) => Ok((wert, next)),
        // At the end of the input syn prefixes its message with
        // "unexpected end of input, ". That is reproduced here so that the
        // message does not change (`list_dx_test::test_cxx_unexpected_eof`).
        None if cursor.eof() => Err(ParseError::at_cursor(
            cursor,
            format!("unexpected end of input, {}", T::erwartet()),
        )),
        None => Err(ParseError::at_cursor(cursor, T::erwartet())),
    }
}

impl SingleToken for proc_macro2::Ident {
    fn take(cursor: Cursor<'_>) -> Option<(Self, Cursor<'_>)> {
        // `impl Parse for Ident` rejects keywords (`accept_as_ident`).
        // The difference to `any_ident` hinges on exactly that.
        let (id, next) = cursor.ident()?;
        if akzeptiert_als_ident(&id.to_string()) {
            Some((id, next))
        } else {
            None
        }
    }
    fn erwartet() -> &'static str {
        "expected identifier"
    }
}

/// The keywords that `syn` does not let pass as an ordinary identifier
/// (`syn::ext::IdentExt::parse_any` bypasses this).
fn akzeptiert_als_ident(s: &str) -> bool {
    !matches!(
        s,
        "abstract"
            | "as"
            | "async"
            | "await"
            | "become"
            | "box"
            | "break"
            | "const"
            | "continue"
            | "crate"
            | "do"
            | "dyn"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "final"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "macro"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "override"
            | "priv"
            | "pub"
            | "ref"
            | "return"
            | "Self"
            | "self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "try"
            | "type"
            | "typeof"
            | "unsafe"
            | "unsized"
            | "use"
            | "virtual"
            | "where"
            | "while"
            | "yield"
    )
}

impl SingleToken for syn::LitBool {
    fn take(cursor: Cursor<'_>) -> Option<(Self, Cursor<'_>)> {
        // A `LitBool` is not a literal but an ident `true`/`false`.
        let (id, next) = cursor.ident()?;
        let s = id.to_string();
        if s == "true" || s == "false" {
            Some((
                syn::LitBool {
                    value: s == "true",
                    span: id.span(),
                },
                next,
            ))
        } else {
            None
        }
    }
    fn erwartet() -> &'static str {
        "expected boolean literal"
    }
}

/// Reads a literal, including a leading minus sign.
///
/// `-5` is a `LitInt` made of TWO cursor tokens; syn handles that in
/// `parse_negative_lit`. Without this step `i32`, `f64` and relatives lose
/// the ability to read negative values.
fn lit_mit_vorzeichen(cursor: Cursor<'_>) -> Option<(syn::Lit, Cursor<'_>)> {
    if let Some((p, nach_minus)) = cursor.punct() {
        if p.as_char() == '-' {
            let (lit, next) = nach_minus.literal()?;
            let mit_minus = format!("-{}", lit);
            // Only numbers may carry a sign.
            return match syn::Lit::new(lit) {
                syn::Lit::Int(_) | syn::Lit::Float(_) => {
                    let mut neu: proc_macro2::Literal = mit_minus.parse().ok()?;
                    neu.set_span(p.span());
                    Some((syn::Lit::new(neu), next))
                }
                _ => None,
            };
        }
    }
    let (lit, next) = cursor.literal()?;
    Some((syn::Lit::new(lit), next))
}

macro_rules! einzeltoken_literal {
    ($typ:ty, $variante:ident, $msg:literal) => {
        impl SingleToken for $typ {
            fn take(cursor: Cursor<'_>) -> Option<(Self, Cursor<'_>)> {
                match lit_mit_vorzeichen(cursor)? {
                    (syn::Lit::$variante(l), next) => Some((l, next)),
                    _ => None,
                }
            }
            fn erwartet() -> &'static str {
                $msg
            }
        }
    };
}

einzeltoken_literal!(syn::LitStr, Str, "expected string literal");
einzeltoken_literal!(syn::LitInt, Int, "expected integer literal");
einzeltoken_literal!(syn::LitFloat, Float, "expected floating point literal");
einzeltoken_literal!(syn::LitChar, Char, "expected character literal");
einzeltoken_literal!(syn::LitByte, Byte, "expected byte literal");
