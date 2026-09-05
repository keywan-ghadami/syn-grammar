use crate::rt::{
    parse_syn, parse_with, step, take_single, ParseContext, ParseError, Stream, StreamResult,
};
use syn::spanned::Spanned;
use syn::Ident;
use syn_grammar_model::model::types::{Identifier, SpannedValue, StringLiteral};

// The single-token builtins keep reading via the cursor - that is O(1) and
// needs no stream. `step` runs them inside a `step` episode and advances
// the stream by exactly their result.

pub fn parse_ident_impl<'a>(
    input: &Stream<'a>,
    ctx: &mut ParseContext<'a>,
) -> StreamResult<'a, Identifier> {
    let t = step(input, take_single::<syn::Ident>)?;
    ctx.record_span(t.span())
        .map_err(|e: syn::Error| ParseError::new(t.span(), e.to_string()))?;
    Ok(Identifier::new(t.to_string(), t.span()))
}

pub fn parse_string_impl<'a>(
    input: &Stream<'a>,
    ctx: &mut ParseContext<'a>,
) -> StreamResult<'a, StringLiteral> {
    let lit = step(input, take_single::<syn::LitStr>)?;
    ctx.record_span(lit.span())
        .map_err(|e: syn::Error| ParseError::new(lit.span(), e.to_string()))?;
    Ok(StringLiteral::new(lit.value(), lit.span()))
}

pub fn parse_char_impl<'a>(
    input: &Stream<'a>,
    ctx: &mut ParseContext<'a>,
) -> StreamResult<'a, char> {
    let lit = step(input, take_single::<syn::LitChar>)?;
    ctx.record_span(lit.span())
        .map_err(|e: syn::Error| ParseError::new(lit.span(), e.to_string()))?;
    Ok(lit.value())
}

pub fn parse_bool_impl<'a>(
    input: &Stream<'a>,
    ctx: &mut ParseContext<'a>,
) -> StreamResult<'a, bool> {
    let lit = step(input, take_single::<syn::LitBool>)?;
    ctx.record_span(lit.span())
        .map_err(|e: syn::Error| ParseError::new(lit.span(), e.to_string()))?;
    Ok(lit.value)
}

// --- Spanned Primitives ---

pub fn parse_spanned_char_impl<'a>(
    input: &Stream<'a>,
    ctx: &mut ParseContext<'a>,
) -> StreamResult<'a, SpannedValue<char>> {
    let lit = step(input, take_single::<syn::LitChar>)?;
    ctx.record_span(lit.span())
        .map_err(|e: syn::Error| ParseError::new(lit.span(), e.to_string()))?;
    Ok(SpannedValue::new(lit.value(), lit.span()))
}

pub fn parse_spanned_bool_impl<'a>(
    input: &Stream<'a>,
    ctx: &mut ParseContext<'a>,
) -> StreamResult<'a, SpannedValue<bool>> {
    let lit = step(input, take_single::<syn::LitBool>)?;
    ctx.record_span(lit.span())
        .map_err(|e: syn::Error| ParseError::new(lit.span(), e.to_string()))?;
    Ok(SpannedValue::new(lit.value, lit.span()))
}

macro_rules! impl_int_builtin {
    ($name:ident, $spanned_name:ident, $ty:ty) => {
        pub fn $name<'a>(input: &Stream<'a>, ctx: &mut ParseContext<'a>) -> StreamResult<'a, $ty> {
            let lit = step(input, take_single::<syn::LitInt>)?;
            let val = lit
                .base10_parse::<$ty>()
                .map_err(|e: syn::Error| ParseError::new(lit.span(), e.to_string()))?;
            ctx.record_span(lit.span())
                .map_err(|e: syn::Error| ParseError::new(lit.span(), e.to_string()))?;
            Ok(val)
        }

        pub fn $spanned_name<'a>(
            input: &Stream<'a>,
            ctx: &mut ParseContext<'a>,
        ) -> StreamResult<'a, SpannedValue<$ty>> {
            let lit = step(input, take_single::<syn::LitInt>)?;
            let val = lit
                .base10_parse::<$ty>()
                .map_err(|e: syn::Error| ParseError::new(lit.span(), e.to_string()))?;
            ctx.record_span(lit.span())
                .map_err(|e: syn::Error| ParseError::new(lit.span(), e.to_string()))?;
            Ok(SpannedValue::new(val, lit.span()))
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
        pub fn $name<'a>(input: &Stream<'a>, ctx: &mut ParseContext<'a>) -> StreamResult<'a, $ty> {
            let lit = step(input, take_single::<syn::LitFloat>)?;
            let val = lit
                .base10_parse::<$ty>()
                .map_err(|e: syn::Error| ParseError::new(lit.span(), e.to_string()))?;
            ctx.record_span(lit.span())
                .map_err(|e: syn::Error| ParseError::new(lit.span(), e.to_string()))?;
            Ok(val)
        }

        pub fn $spanned_name<'a>(
            input: &Stream<'a>,
            ctx: &mut ParseContext<'a>,
        ) -> StreamResult<'a, SpannedValue<$ty>> {
            let lit = step(input, take_single::<syn::LitFloat>)?;
            let val = lit
                .base10_parse::<$ty>()
                .map_err(|e: syn::Error| ParseError::new(lit.span(), e.to_string()))?;
            ctx.record_span(lit.span())
                .map_err(|e: syn::Error| ParseError::new(lit.span(), e.to_string()))?;
            Ok(SpannedValue::new(val, lit.span()))
        }
    };
}

impl_float_builtin!(parse_f32_impl, parse_spanned_f32_impl, f32);
impl_float_builtin!(parse_f64_impl, parse_spanned_f64_impl, f64);

// Alternative Bases
pub fn parse_hex_literal_impl<'a>(
    input: &Stream<'a>,
    ctx: &mut ParseContext<'a>,
) -> StreamResult<'a, u64> {
    parse_u64_impl(input, ctx)
}

pub fn parse_oct_literal_impl<'a>(
    input: &Stream<'a>,
    ctx: &mut ParseContext<'a>,
) -> StreamResult<'a, u64> {
    parse_u64_impl(input, ctx)
}

pub fn parse_bin_literal_impl<'a>(
    input: &Stream<'a>,
    ctx: &mut ParseContext<'a>,
) -> StreamResult<'a, u64> {
    parse_u64_impl(input, ctx)
}

// Single-token builtins: O(1) directly via the cursor.
macro_rules! impl_single_token_builtin {
    ($name:ident, $ty:ty) => {
        pub fn $name<'a>(input: &Stream<'a>, ctx: &mut ParseContext<'a>) -> StreamResult<'a, $ty> {
            let t = step(input, take_single::<$ty>)?;
            ctx.record_span(t.span())
                .map_err(|e: syn::Error| ParseError::new(t.span(), e.to_string()))?;
            Ok(t)
        }
    };
}

// Real syn AST types. Since ADR 15, stage 3 this is an ordinary `parse`
// call on the existing stream - O(length of the type) instead of O(rest).
macro_rules! impl_syn_builtin {
    ($name:ident, $ty:ty) => {
        pub fn $name<'a>(input: &Stream<'a>, ctx: &mut ParseContext<'a>) -> StreamResult<'a, $ty> {
            let t = parse_syn::<$ty>(input)?;
            ctx.record_span(t.span())
                .map_err(|e: syn::Error| ParseError::new(t.span(), e.to_string()))?;
            Ok(t)
        }
    };
}

impl_syn_builtin!(parse_rust_type_impl, syn::Type);
impl_syn_builtin!(parse_rust_block_impl, syn::Block);
impl_single_token_builtin!(parse_lit_str_impl, syn::LitStr);
impl_single_token_builtin!(parse_lit_int_impl, syn::LitInt);
impl_single_token_builtin!(parse_lit_char_impl, syn::LitChar);
impl_single_token_builtin!(parse_lit_bool_impl, syn::LitBool);
impl_single_token_builtin!(parse_lit_float_impl, syn::LitFloat);
/// `any_ident` accepts - unlike `ident` - keywords as well.
///
/// syn's `Ident` parser rejects `self`, `type`, `fn` etc. Previously `any_ident`
/// used exactly that parser and was thus identical to `ident`; grammars such as
/// the cxx one (`fn f(self: Pin<&mut T>)`) failed because of that. `Ident::parse_any`
/// from `syn::ext::IdentExt` is the intended way.
pub fn parse_any_ident_impl<'a>(
    input: &Stream<'a>,
    ctx: &mut ParseContext<'a>,
) -> StreamResult<'a, Ident> {
    // `Ident::parse_any` simply means: any ident token, keywords included.
    // That is `cursor.ident()` without the `accept_as_ident` filter, in O(1).
    // Appears in every function argument in cxx.
    let t = step(input, |cursor| match cursor.ident() {
        Some(x) => Ok(x),
        None => Err(ParseError::expecting(cursor, "identifier")),
    })?;
    ctx.record_span(t.span())
        .map_err(|e: syn::Error| ParseError::new(t.span(), e.to_string()))?;
    Ok(t)
}
impl_syn_builtin!(parse_visibility_impl, syn::Visibility);
impl_syn_builtin!(parse_generics_impl, syn::Generics);
impl_syn_builtin!(parse_return_type_impl, syn::ReturnType);

// --- Custom Parsers (Field, Attribute, Block::parse_within) ---

pub fn parse_named_field_impl<'a>(
    input: &Stream<'a>,
    ctx: &mut ParseContext<'a>,
) -> StreamResult<'a, syn::Field> {
    let t = parse_with(input, syn::Field::parse_named)?;
    ctx.record_span(t.span())
        .map_err(|e: syn::Error| ParseError::new(t.span(), e.to_string()))?;
    Ok(t)
}

pub fn parse_unnamed_field_impl<'a>(
    input: &Stream<'a>,
    ctx: &mut ParseContext<'a>,
) -> StreamResult<'a, syn::Field> {
    let t = parse_with(input, syn::Field::parse_unnamed)?;
    ctx.record_span(t.span())
        .map_err(|e: syn::Error| ParseError::new(t.span(), e.to_string()))?;
    Ok(t)
}

pub fn parse_outer_attrs_impl<'a>(
    input: &Stream<'a>,
    ctx: &mut ParseContext<'a>,
) -> StreamResult<'a, Vec<syn::Attribute>> {
    let attrs = parse_with(input, syn::Attribute::parse_outer)?;
    if let Some(last) = attrs.last() {
        ctx.record_span(last.span())
            .map_err(|e: syn::Error| ParseError::new(last.span(), e.to_string()))?;
    }
    Ok(attrs)
}

pub fn parse_statements_impl<'a>(
    input: &Stream<'a>,
    ctx: &mut ParseContext<'a>,
) -> StreamResult<'a, Vec<syn::Stmt>> {
    let stmts = parse_with(input, syn::Block::parse_within)?;
    if let Some(last) = stmts.last() {
        ctx.record_span(last.span())
            .map_err(|e: syn::Error| ParseError::new(last.span(), e.to_string()))?;
    }
    Ok(stmts)
}

/// A Rust pattern (`syn::Pat`).
///
/// `syn::Pat` deliberately has no `impl Parse` - syn demands the decision
/// between `parse_single` and `parse_multi`, because `A | B` is, depending on
/// context, an or-pattern or two separate patterns. So `Pat` was not reachable
/// via the `syn::` path in `codegen/pattern.rs`: everything there goes through
/// `rt::parse_syn::<T: Parse>`. Every grammar with Rust patterns
/// (`let`, `match`, function parameters) was stuck on this gap.
///
/// The choice is `parse_multi_with_leading_vert` - the form that `match` arms
/// use and that includes `parse_single` as a special case.
pub fn parse_pat_impl<'a>(
    input: &Stream<'a>,
    ctx: &mut ParseContext<'a>,
) -> StreamResult<'a, syn::Pat> {
    let pat = parse_with(input, syn::Pat::parse_multi_with_leading_vert)?;
    ctx.record_span(pat.span())
        .map_err(|e: syn::Error| ParseError::new(pat.span(), e.to_string()))?;
    Ok(pat)
}

/// Inner attributes (`#![...]`).
///
/// Counterpart of `outer_attrs`. Previously there was only `Attribute::parse_outer`,
/// so module and crate attributes were not parseable.
pub fn parse_inner_attrs_impl<'a>(
    input: &Stream<'a>,
    ctx: &mut ParseContext<'a>,
) -> StreamResult<'a, Vec<syn::Attribute>> {
    let attrs = parse_with(input, syn::Attribute::parse_inner)?;
    if let Some(last) = attrs.last() {
        ctx.record_span(last.span())
            .map_err(|e: syn::Error| ParseError::new(last.span(), e.to_string()))?;
    }
    Ok(attrs)
}

/// A byte literal (`b'A'`).
///
/// `any_byte` already returns `syn::LitByte`, but was not named like the rest of
/// the `lit_*` family. `lit_byte` closes the gap in the naming scheme without
/// removing `any_byte`.
pub fn parse_lit_byte_impl<'a>(
    input: &Stream<'a>,
    ctx: &mut ParseContext<'a>,
) -> StreamResult<'a, syn::LitByte> {
    let t = step(input, take_single::<syn::LitByte>)?;
    ctx.record_span(t.span())
        .map_err(|e: syn::Error| ParseError::new(t.span(), e.to_string()))?;
    Ok(t)
}
