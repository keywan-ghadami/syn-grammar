use crate::rt::{self, ParseContext};
use syn::parse::ParseStream;
use syn::spanned::Spanned;
use syn::Result;

pub fn parse_ident_impl(input: ParseStream, ctx: &mut ParseContext) -> Result<syn::Ident> {
    let t = rt::parse_ident(input)?;
    ctx.record_span(t.span());
    Ok(t)
}

pub fn parse_integer_impl(input: ParseStream, ctx: &mut ParseContext) -> Result<i32> {
    let lit = input.parse::<syn::LitInt>()?;
    ctx.record_span(lit.span());
    lit.base10_parse()
}

pub fn parse_string_impl(input: ParseStream, ctx: &mut ParseContext) -> Result<String> {
    let lit = input.parse::<syn::LitStr>()?;
    ctx.record_span(lit.span());
    Ok(lit.value())
}

pub fn parse_float_impl(input: ParseStream, ctx: &mut ParseContext) -> Result<f64> {
    let lit = input.parse::<syn::LitFloat>()?;
    ctx.record_span(lit.span());
    lit.base10_parse()
}

pub fn parse_rust_type_impl(input: ParseStream, ctx: &mut ParseContext) -> Result<syn::Type> {
    let t: syn::Type = input.parse()?;
    // Type is Spanned
    ctx.record_span(t.span());
    Ok(t)
}

pub fn parse_rust_block_impl(input: ParseStream, ctx: &mut ParseContext) -> Result<syn::Block> {
    let b: syn::Block = input.parse()?;
    ctx.record_span(b.span());
    Ok(b)
}

pub fn parse_lit_str_impl(input: ParseStream, ctx: &mut ParseContext) -> Result<syn::LitStr> {
    let t: syn::LitStr = input.parse()?;
    ctx.record_span(t.span());
    Ok(t)
}

pub fn parse_lit_int_impl(input: ParseStream, ctx: &mut ParseContext) -> Result<syn::LitInt> {
    let t: syn::LitInt = input.parse()?;
    ctx.record_span(t.span());
    Ok(t)
}

pub fn parse_lit_char_impl(input: ParseStream, ctx: &mut ParseContext) -> Result<syn::LitChar> {
    let t: syn::LitChar = input.parse()?;
    ctx.record_span(t.span());
    Ok(t)
}

pub fn parse_lit_bool_impl(input: ParseStream, ctx: &mut ParseContext) -> Result<syn::LitBool> {
    let t: syn::LitBool = input.parse()?;
    ctx.record_span(t.span());
    Ok(t)
}

pub fn parse_lit_float_impl(input: ParseStream, ctx: &mut ParseContext) -> Result<syn::LitFloat> {
    let t: syn::LitFloat = input.parse()?;
    ctx.record_span(t.span());
    Ok(t)
}

pub fn parse_outer_attrs_impl(
    input: ParseStream,
    ctx: &mut ParseContext,
) -> Result<Vec<syn::Attribute>> {
    let attrs = syn::Attribute::parse_outer(input)?;
    if let Some(last) = attrs.last() {
        ctx.record_span(last.span());
    } else {
        // No attributes parsed, so no span to record.
        // Logic might be slightly off if we wanted to record the "absence" of span,
        // but typically whitespace check is against *something* that was consumed.
    }
    Ok(attrs)
}

// Spanned variants
pub fn parse_spanned_int_lit_impl(
    input: ParseStream,
    ctx: &mut ParseContext,
) -> Result<(i32, proc_macro2::Span)> {
    let l = input.parse::<syn::LitInt>()?;
    ctx.record_span(l.span());
    Ok((l.base10_parse::<i32>()?, l.span()))
}

pub fn parse_spanned_string_lit_impl(
    input: ParseStream,
    ctx: &mut ParseContext,
) -> Result<(String, proc_macro2::Span)> {
    let l = input.parse::<syn::LitStr>()?;
    ctx.record_span(l.span());
    Ok((l.value(), l.span()))
}

pub fn parse_spanned_float_lit_impl(
    input: ParseStream,
    ctx: &mut ParseContext,
) -> Result<(f64, proc_macro2::Span)> {
    let l = input.parse::<syn::LitFloat>()?;
    ctx.record_span(l.span());
    Ok((l.base10_parse::<f64>()?, l.span()))
}

pub fn parse_spanned_bool_lit_impl(
    input: ParseStream,
    ctx: &mut ParseContext,
) -> Result<(bool, proc_macro2::Span)> {
    let l = input.parse::<syn::LitBool>()?;
    ctx.record_span(l.span());
    Ok((l.value, l.span()))
}

pub fn parse_spanned_char_lit_impl(
    input: ParseStream,
    ctx: &mut ParseContext,
) -> Result<(char, proc_macro2::Span)> {
    let l = input.parse::<syn::LitChar>()?;
    ctx.record_span(l.span());
    Ok((l.value(), l.span()))
}
