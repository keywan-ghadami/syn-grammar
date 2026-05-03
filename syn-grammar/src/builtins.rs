use crate::rt::{invoke_syn_parser, ParseContext, ParseError, ParseResult};
use syn::buffer::Cursor;
use syn::spanned::Spanned;
use syn::Ident;
use syn_grammar_model::model::types::{Identifier, SpannedValue, StringLiteral};

pub fn parse_ident_impl<'a>(cursor: Cursor<'a>, ctx: &mut ParseContext) -> ParseResult<'a, Identifier> {
    let (t, next) = invoke_syn_parser::<syn::Ident>(cursor)?;
    ctx.record_span(t.span()).map_err(|e: syn::Error| ParseError::new(t.span(), e.to_string()))?;
    Ok((Identifier::new(t.to_string(), t.span()), next))
}

pub fn parse_string_impl<'a>(cursor: Cursor<'a>, ctx: &mut ParseContext) -> ParseResult<'a, StringLiteral> {
    let (lit, next) = invoke_syn_parser::<syn::LitStr>(cursor)?;
    ctx.record_span(lit.span()).map_err(|e: syn::Error| ParseError::new(lit.span(), e.to_string()))?;
    Ok((StringLiteral::new(lit.value(), lit.span()), next))
}

pub fn parse_char_impl<'a>(cursor: Cursor<'a>, ctx: &mut ParseContext) -> ParseResult<'a, char> {
    let (lit, next) = invoke_syn_parser::<syn::LitChar>(cursor)?;
    ctx.record_span(lit.span()).map_err(|e: syn::Error| ParseError::new(lit.span(), e.to_string()))?;
    Ok((lit.value(), next))
}

pub fn parse_bool_impl<'a>(cursor: Cursor<'a>, ctx: &mut ParseContext) -> ParseResult<'a, bool> {
    let (lit, next) = invoke_syn_parser::<syn::LitBool>(cursor)?;
    ctx.record_span(lit.span()).map_err(|e: syn::Error| ParseError::new(lit.span(), e.to_string()))?;
    Ok((lit.value, next))
}

// --- Spanned Primitives ---

pub fn parse_spanned_char_impl<'a>(cursor: Cursor<'a>, ctx: &mut ParseContext) -> ParseResult<'a, SpannedValue<char>> {
    let (lit, next) = invoke_syn_parser::<syn::LitChar>(cursor)?;
    ctx.record_span(lit.span()).map_err(|e: syn::Error| ParseError::new(lit.span(), e.to_string()))?;
    Ok((SpannedValue::new(lit.value(), lit.span()), next))
}

pub fn parse_spanned_bool_impl<'a>(cursor: Cursor<'a>, ctx: &mut ParseContext) -> ParseResult<'a, SpannedValue<bool>> {
    let (lit, next) = invoke_syn_parser::<syn::LitBool>(cursor)?;
    ctx.record_span(lit.span()).map_err(|e: syn::Error| ParseError::new(lit.span(), e.to_string()))?;
    Ok((SpannedValue::new(lit.value, lit.span()), next))
}

macro_rules! impl_int_builtin {
    ($name:ident, $spanned_name:ident, $ty:ty) => {
        pub fn $name<'a>(cursor: Cursor<'a>, ctx: &mut ParseContext) -> ParseResult<'a, $ty> {
            let (lit, next) = invoke_syn_parser::<syn::LitInt>(cursor)?;
            let val = lit.base10_parse::<$ty>().map_err(|e: syn::Error| ParseError::new(lit.span(), e.to_string()))?;
            ctx.record_span(lit.span()).map_err(|e: syn::Error| ParseError::new(lit.span(), e.to_string()))?;
            Ok((val, next))
        }

        pub fn $spanned_name<'a>(cursor: Cursor<'a>, ctx: &mut ParseContext) -> ParseResult<'a, SpannedValue<$ty>> {
            let (lit, next) = invoke_syn_parser::<syn::LitInt>(cursor)?;
            let val = lit.base10_parse::<$ty>().map_err(|e: syn::Error| ParseError::new(lit.span(), e.to_string()))?;
            ctx.record_span(lit.span()).map_err(|e: syn::Error| ParseError::new(lit.span(), e.to_string()))?;
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
        pub fn $name<'a>(cursor: Cursor<'a>, ctx: &mut ParseContext) -> ParseResult<'a, $ty> {
            let (lit, next) = invoke_syn_parser::<syn::LitFloat>(cursor)?;
            let val = lit.base10_parse::<$ty>().map_err(|e: syn::Error| ParseError::new(lit.span(), e.to_string()))?;
            ctx.record_span(lit.span()).map_err(|e: syn::Error| ParseError::new(lit.span(), e.to_string()))?;
            Ok((val, next))
        }

        pub fn $spanned_name<'a>(cursor: Cursor<'a>, ctx: &mut ParseContext) -> ParseResult<'a, SpannedValue<$ty>> {
            let (lit, next) = invoke_syn_parser::<syn::LitFloat>(cursor)?;
            let val = lit.base10_parse::<$ty>().map_err(|e: syn::Error| ParseError::new(lit.span(), e.to_string()))?;
            ctx.record_span(lit.span()).map_err(|e: syn::Error| ParseError::new(lit.span(), e.to_string()))?;
            Ok((SpannedValue::new(val, lit.span()), next))
        }
    };
}

impl_float_builtin!(parse_f32_impl, parse_spanned_f32_impl, f32);
impl_float_builtin!(parse_f64_impl, parse_spanned_f64_impl, f64);

// Alternative Bases
pub fn parse_hex_literal_impl<'a>(cursor: Cursor<'a>, ctx: &mut ParseContext) -> ParseResult<'a, u64> {
    parse_u64_impl(cursor, ctx)
}

pub fn parse_oct_literal_impl<'a>(cursor: Cursor<'a>, ctx: &mut ParseContext) -> ParseResult<'a, u64> {
    parse_u64_impl(cursor, ctx)
}

pub fn parse_bin_literal_impl<'a>(cursor: Cursor<'a>, ctx: &mut ParseContext) -> ParseResult<'a, u64> {
    parse_u64_impl(cursor, ctx)
}

// Syn Specific Built-ins
macro_rules! impl_syn_builtin {
    ($name:ident, $ty:ty) => {
        pub fn $name<'a>(cursor: Cursor<'a>, ctx: &mut ParseContext) -> ParseResult<'a, $ty> {
            let (t, next) = invoke_syn_parser::<$ty>(cursor)?;
            ctx.record_span(t.span()).map_err(|e: syn::Error| ParseError::new(t.span(), e.to_string()))?;
            Ok((t, next))
        }
    };
}

impl_syn_builtin!(parse_rust_type_impl, syn::Type);
impl_syn_builtin!(parse_rust_block_impl, syn::Block);
impl_syn_builtin!(parse_lit_str_impl, syn::LitStr);
impl_syn_builtin!(parse_lit_int_impl, syn::LitInt);
impl_syn_builtin!(parse_lit_char_impl, syn::LitChar);
impl_syn_builtin!(parse_lit_bool_impl, syn::LitBool);
impl_syn_builtin!(parse_lit_float_impl, syn::LitFloat);
impl_syn_builtin!(parse_any_ident_impl, Ident);
impl_syn_builtin!(parse_named_field_impl, syn::Field); 
impl_syn_builtin!(parse_visibility_impl, syn::Visibility);
impl_syn_builtin!(parse_generics_impl, syn::Generics);
impl_syn_builtin!(parse_return_type_impl, syn::ReturnType);

pub fn parse_outer_attrs_impl<'a>(cursor: Cursor<'a>, ctx: &mut ParseContext) -> ParseResult<'a, Vec<syn::Attribute>> {
    let (attrs, next) = invoke_syn_parser::<syn::Attribute>(cursor).map(|(a, c)| (vec![a], c))?; 
    if let Some(last) = attrs.last() {
        ctx.record_span(last.span()).map_err(|e: syn::Error| ParseError::new(last.span(), e.to_string()))?;
    }
    Ok((attrs, next))
}

pub fn parse_statements_impl<'a>(cursor: Cursor<'a>, ctx: &mut ParseContext) -> ParseResult<'a, Vec<syn::Stmt>> {
    let (block, next) = invoke_syn_parser::<syn::Block>(cursor)?;
    if let Some(last) = block.stmts.last() {
        ctx.record_span(last.span()).map_err(|e: syn::Error| ParseError::new(last.span(), e.to_string()))?;
    }
    Ok((block.stmts, next))
}
