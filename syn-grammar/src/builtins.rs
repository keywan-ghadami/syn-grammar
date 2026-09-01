use crate::rt::{invoke_syn_parser, take_single, ParseContext, ParseError, ParseResult};
use syn::buffer::Cursor;
use syn::spanned::Spanned;
use syn::Ident;
use syn_grammar_model::model::types::{Identifier, SpannedValue, StringLiteral};

// Builtins fuer Typen ohne `impl Parse` (`Field`, `Attribute`, `Pat`) oder mit
// Sonderparser (`Block::parse_within`, `Ident::parse_any`) gehen ueber denselben
// Rumpf wie `invoke_syn_parser`. Frueher stand hier eine textgleiche Kopie der
// Materialisierungslogik.
use crate::rt::invoke_parser_fn as invoke_custom_parser;

pub fn parse_ident_impl<'a>(
    cursor: Cursor<'a>,
    ctx: &mut ParseContext<'a>,
) -> ParseResult<'a, Identifier> {
    let (t, next) = take_single::<syn::Ident>(cursor)?;
    ctx.record_span(t.span())
        .map_err(|e: syn::Error| ParseError::new(t.span(), e.to_string()))?;
    Ok((Identifier::new(t.to_string(), t.span()), next))
}

pub fn parse_string_impl<'a>(
    cursor: Cursor<'a>,
    ctx: &mut ParseContext<'a>,
) -> ParseResult<'a, StringLiteral> {
    let (lit, next) = take_single::<syn::LitStr>(cursor)?;
    ctx.record_span(lit.span())
        .map_err(|e: syn::Error| ParseError::new(lit.span(), e.to_string()))?;
    Ok((StringLiteral::new(lit.value(), lit.span()), next))
}

pub fn parse_char_impl<'a>(
    cursor: Cursor<'a>,
    ctx: &mut ParseContext<'a>,
) -> ParseResult<'a, char> {
    let (lit, next) = take_single::<syn::LitChar>(cursor)?;
    ctx.record_span(lit.span())
        .map_err(|e: syn::Error| ParseError::new(lit.span(), e.to_string()))?;
    Ok((lit.value(), next))
}

pub fn parse_bool_impl<'a>(
    cursor: Cursor<'a>,
    ctx: &mut ParseContext<'a>,
) -> ParseResult<'a, bool> {
    let (lit, next) = take_single::<syn::LitBool>(cursor)?;
    ctx.record_span(lit.span())
        .map_err(|e: syn::Error| ParseError::new(lit.span(), e.to_string()))?;
    Ok((lit.value, next))
}

// --- Spanned Primitives ---

pub fn parse_spanned_char_impl<'a>(
    cursor: Cursor<'a>,
    ctx: &mut ParseContext<'a>,
) -> ParseResult<'a, SpannedValue<char>> {
    let (lit, next) = take_single::<syn::LitChar>(cursor)?;
    ctx.record_span(lit.span())
        .map_err(|e: syn::Error| ParseError::new(lit.span(), e.to_string()))?;
    Ok((SpannedValue::new(lit.value(), lit.span()), next))
}

pub fn parse_spanned_bool_impl<'a>(
    cursor: Cursor<'a>,
    ctx: &mut ParseContext<'a>,
) -> ParseResult<'a, SpannedValue<bool>> {
    let (lit, next) = take_single::<syn::LitBool>(cursor)?;
    ctx.record_span(lit.span())
        .map_err(|e: syn::Error| ParseError::new(lit.span(), e.to_string()))?;
    Ok((SpannedValue::new(lit.value, lit.span()), next))
}

macro_rules! impl_int_builtin {
    ($name:ident, $spanned_name:ident, $ty:ty) => {
        pub fn $name<'a>(cursor: Cursor<'a>, ctx: &mut ParseContext<'a>) -> ParseResult<'a, $ty> {
            let (lit, next) = take_single::<syn::LitInt>(cursor)?;
            let val = lit
                .base10_parse::<$ty>()
                .map_err(|e: syn::Error| ParseError::new(lit.span(), e.to_string()))?;
            ctx.record_span(lit.span())
                .map_err(|e: syn::Error| ParseError::new(lit.span(), e.to_string()))?;
            Ok((val, next))
        }

        pub fn $spanned_name<'a>(
            cursor: Cursor<'a>,
            ctx: &mut ParseContext<'a>,
        ) -> ParseResult<'a, SpannedValue<$ty>> {
            let (lit, next) = take_single::<syn::LitInt>(cursor)?;
            let val = lit
                .base10_parse::<$ty>()
                .map_err(|e: syn::Error| ParseError::new(lit.span(), e.to_string()))?;
            ctx.record_span(lit.span())
                .map_err(|e: syn::Error| ParseError::new(lit.span(), e.to_string()))?;
            Ok((SpannedValue::new(val, lit.span()), next))
        }
    };
}

impl_int_builtin!(parse_i8_impl, parse_spanned_i8_impl, i8);
impl_int_builtin!(parse_i16_impl, parse_spanned_i16_impl, i16);
impl_int_builtin!(parse_i32_impl, parse_spanned_i32_impl, i32);
impl_int_builtin!(parse_i64_impl, parse_spanned_i64_impl, i64);
impl_int_builtin!(parse_i128_impl, parse_spanned_i128_impl, i128);
impl_int_builtin!(parse_isize_impl, parse_spanned_isize_impl, isize);

impl_int_builtin!(parse_u8_impl, parse_spanned_u8_impl, u8);
impl_int_builtin!(parse_u16_impl, parse_spanned_u16_impl, u16);
impl_int_builtin!(parse_u32_impl, parse_spanned_u32_impl, u32);
impl_int_builtin!(parse_u64_impl, parse_spanned_u64_impl, u64);
impl_int_builtin!(parse_u128_impl, parse_spanned_u128_impl, u128);
impl_int_builtin!(parse_usize_impl, parse_spanned_usize_impl, usize);

macro_rules! impl_float_builtin {
    ($name:ident, $spanned_name:ident, $ty:ty) => {
        pub fn $name<'a>(cursor: Cursor<'a>, ctx: &mut ParseContext<'a>) -> ParseResult<'a, $ty> {
            let (lit, next) = take_single::<syn::LitFloat>(cursor)?;
            let val = lit
                .base10_parse::<$ty>()
                .map_err(|e: syn::Error| ParseError::new(lit.span(), e.to_string()))?;
            ctx.record_span(lit.span())
                .map_err(|e: syn::Error| ParseError::new(lit.span(), e.to_string()))?;
            Ok((val, next))
        }

        pub fn $spanned_name<'a>(
            cursor: Cursor<'a>,
            ctx: &mut ParseContext<'a>,
        ) -> ParseResult<'a, SpannedValue<$ty>> {
            let (lit, next) = take_single::<syn::LitFloat>(cursor)?;
            let val = lit
                .base10_parse::<$ty>()
                .map_err(|e: syn::Error| ParseError::new(lit.span(), e.to_string()))?;
            ctx.record_span(lit.span())
                .map_err(|e: syn::Error| ParseError::new(lit.span(), e.to_string()))?;
            Ok((SpannedValue::new(val, lit.span()), next))
        }
    };
}

impl_float_builtin!(parse_f32_impl, parse_spanned_f32_impl, f32);
impl_float_builtin!(parse_f64_impl, parse_spanned_f64_impl, f64);

// Alternative Bases
pub fn parse_hex_literal_impl<'a>(
    cursor: Cursor<'a>,
    ctx: &mut ParseContext<'a>,
) -> ParseResult<'a, u64> {
    parse_u64_impl(cursor, ctx)
}

pub fn parse_oct_literal_impl<'a>(
    cursor: Cursor<'a>,
    ctx: &mut ParseContext<'a>,
) -> ParseResult<'a, u64> {
    parse_u64_impl(cursor, ctx)
}

pub fn parse_bin_literal_impl<'a>(
    cursor: Cursor<'a>,
    ctx: &mut ParseContext<'a>,
) -> ParseResult<'a, u64> {
    parse_u64_impl(cursor, ctx)
}

// Einzeltoken-Builtins: O(1) ueber den Cursor, ohne den Reststrom anzufassen.
macro_rules! impl_einzeltoken_builtin {
    ($name:ident, $ty:ty) => {
        pub fn $name<'a>(cursor: Cursor<'a>, ctx: &mut ParseContext<'a>) -> ParseResult<'a, $ty> {
            let (t, next) = take_single::<$ty>(cursor)?;
            ctx.record_span(t.span())
                .map_err(|e: syn::Error| ParseError::new(t.span(), e.to_string()))?;
            Ok((t, next))
        }
    };
}

// Echte syn-AST-Typen: die Bruecke bleibt noetig, weil sie mehrere Tokens
// umspannen und ihre Laenge erst beim Parsen feststeht.
macro_rules! impl_syn_builtin {
    ($name:ident, $ty:ty) => {
        pub fn $name<'a>(cursor: Cursor<'a>, ctx: &mut ParseContext<'a>) -> ParseResult<'a, $ty> {
            let (t, next) = invoke_syn_parser::<$ty>(cursor)?;
            ctx.record_span(t.span())
                .map_err(|e: syn::Error| ParseError::new(t.span(), e.to_string()))?;
            Ok((t, next))
        }
    };
}

impl_syn_builtin!(parse_rust_type_impl, syn::Type);
/// `rust_block` geht ueber den gruppenbegrenzten Weg statt ueber die Bruecke:
/// ein Block ist genau ein `{}`-Token-Tree, der Rest muss nicht angefasst werden.
pub fn parse_rust_block_impl<'a>(
    cursor: Cursor<'a>,
    ctx: &mut ParseContext<'a>,
) -> ParseResult<'a, syn::Block> {
    let (t, next) = crate::rt::take_braced_block(cursor)?;
    ctx.record_span(t.span())
        .map_err(|e: syn::Error| ParseError::new(t.span(), e.to_string()))?;
    Ok((t, next))
}
impl_einzeltoken_builtin!(parse_lit_str_impl, syn::LitStr);
impl_einzeltoken_builtin!(parse_lit_int_impl, syn::LitInt);
impl_einzeltoken_builtin!(parse_lit_char_impl, syn::LitChar);
impl_einzeltoken_builtin!(parse_lit_bool_impl, syn::LitBool);
impl_einzeltoken_builtin!(parse_lit_float_impl, syn::LitFloat);
/// `any_ident` akzeptiert - anders als `ident` - auch Schluesselwoerter.
///
/// syns `Ident`-Parser lehnt `self`, `type`, `fn` usw. ab. Bisher benutzte
/// `any_ident` genau diesen Parser und war damit identisch mit `ident`; Grammatiken
/// wie die von cxx (`fn f(self: Pin<&mut T>)`) scheiterten daran. `Ident::parse_any`
/// aus `syn::ext::IdentExt` ist der dafuer vorgesehene Weg.
pub fn parse_any_ident_impl<'a>(
    cursor: Cursor<'a>,
    ctx: &mut ParseContext<'a>,
) -> ParseResult<'a, Ident> {
    // `Ident::parse_any` heisst schlicht: jedes Ident-Token, auch
    // Schluesselwoerter. Das ist `cursor.ident()` ohne den `accept_as_ident`-
    // Filter - in O(1), ohne den Reststrom zu materialisieren. Steht in cxx in
    // jedem Funktionsargument.
    let (t, next) = match cursor.ident() {
        Some(x) => x,
        None => return Err(ParseError::at_cursor(cursor, "expected identifier")),
    };
    ctx.record_span(t.span())
        .map_err(|e: syn::Error| ParseError::new(t.span(), e.to_string()))?;
    Ok((t, next))
}
impl_syn_builtin!(parse_visibility_impl, syn::Visibility);
impl_syn_builtin!(parse_generics_impl, syn::Generics);
impl_syn_builtin!(parse_return_type_impl, syn::ReturnType);

// --- Custom Parsers (Field, Attribute, Block::parse_within) ---

pub fn parse_named_field_impl<'a>(
    cursor: Cursor<'a>,
    ctx: &mut ParseContext<'a>,
) -> ParseResult<'a, syn::Field> {
    let (t, next) = invoke_custom_parser(cursor, syn::Field::parse_named)?;
    ctx.record_span(t.span())
        .map_err(|e: syn::Error| ParseError::new(t.span(), e.to_string()))?;
    Ok((t, next))
}

pub fn parse_unnamed_field_impl<'a>(
    cursor: Cursor<'a>,
    ctx: &mut ParseContext<'a>,
) -> ParseResult<'a, syn::Field> {
    let (t, next) = invoke_custom_parser(cursor, syn::Field::parse_unnamed)?;
    ctx.record_span(t.span())
        .map_err(|e: syn::Error| ParseError::new(t.span(), e.to_string()))?;
    Ok((t, next))
}

pub fn parse_outer_attrs_impl<'a>(
    cursor: Cursor<'a>,
    ctx: &mut ParseContext<'a>,
) -> ParseResult<'a, Vec<syn::Attribute>> {
    let (attrs, next) = invoke_custom_parser(cursor, syn::Attribute::parse_outer)?;
    if let Some(last) = attrs.last() {
        ctx.record_span(last.span())
            .map_err(|e: syn::Error| ParseError::new(last.span(), e.to_string()))?;
    }
    Ok((attrs, next))
}

pub fn parse_statements_impl<'a>(
    cursor: Cursor<'a>,
    ctx: &mut ParseContext<'a>,
) -> ParseResult<'a, Vec<syn::Stmt>> {
    let (stmts, next) = invoke_custom_parser(cursor, syn::Block::parse_within)?;
    if let Some(last) = stmts.last() {
        ctx.record_span(last.span())
            .map_err(|e: syn::Error| ParseError::new(last.span(), e.to_string()))?;
    }
    Ok((stmts, next))
}

/// Ein Rust-Muster (`syn::Pat`).
///
/// `syn::Pat` hat bewusst kein `impl Parse` - syn verlangt die Entscheidung
/// zwischen `parse_single` und `parse_multi`, weil `A | B` je nach Kontext ein
/// Oder-Muster oder zwei getrennte Muster ist. Damit war `Pat` ueber den
/// `syn::`-Pfad in `codegen/pattern.rs` nicht erreichbar: dort laeuft alles
/// ueber `invoke_syn_parser::<T: Parse>`. Jede Grammatik mit Rust-Mustern
/// (`let`, `match`, Funktionsparameter) hing an dieser Luecke.
///
/// Gewaehlt ist `parse_multi_with_leading_vert` - die Form, die `match`-Arme
/// benutzen und die `parse_single` als Sonderfall einschliesst.
pub fn parse_pat_impl<'a>(
    cursor: Cursor<'a>,
    ctx: &mut ParseContext<'a>,
) -> ParseResult<'a, syn::Pat> {
    let (pat, next) = invoke_custom_parser(cursor, syn::Pat::parse_multi_with_leading_vert)?;
    ctx.record_span(pat.span())
        .map_err(|e: syn::Error| ParseError::new(pat.span(), e.to_string()))?;
    Ok((pat, next))
}

/// Innere Attribute (`#![...]`).
///
/// Gegenstueck zu `outer_attrs`. Es gab bisher nur `Attribute::parse_outer`,
/// womit Modul- und Crate-Attribute nicht parsebar waren.
pub fn parse_inner_attrs_impl<'a>(
    cursor: Cursor<'a>,
    ctx: &mut ParseContext<'a>,
) -> ParseResult<'a, Vec<syn::Attribute>> {
    let (attrs, next) = invoke_custom_parser(cursor, syn::Attribute::parse_inner)?;
    if let Some(last) = attrs.last() {
        ctx.record_span(last.span())
            .map_err(|e: syn::Error| ParseError::new(last.span(), e.to_string()))?;
    }
    Ok((attrs, next))
}

/// Ein Byte-Literal (`b'A'`).
///
/// `any_byte` liefert bereits `syn::LitByte`, hiess aber nicht wie die uebrige
/// `lit_*`-Familie. `lit_byte` schliesst die Luecke im Namensschema, ohne
/// `any_byte` zu entfernen.
pub fn parse_lit_byte_impl<'a>(
    cursor: Cursor<'a>,
    ctx: &mut ParseContext<'a>,
) -> ParseResult<'a, syn::LitByte> {
    let (t, next) = take_single::<syn::LitByte>(cursor)?;
    ctx.record_span(t.span())
        .map_err(|e: syn::Error| ParseError::new(t.span(), e.to_string()))?;
    Ok((t, next))
}
