//! Coverage for the built-in rules that no test had touched before.
//!
//! The catalogue in `syn-grammar-macros/src/backend.rs` has 60 entries; before
//! this file 39 of them were untested - among them the complete `spanned_*`
//! family, all token filters and the syn interop builtins. Exactly in this zone
//! were the two defects the review found (wrong return type for
//! `digit`/`hex_digit`/`oct_digit`, dead catalogue entry `fail`). An untested
//! builtin is a promise without cover.
//!
//! Bundled by family instead of 39 individual tests: one grammar per family,
//! with one rule per builtin in it.

use syn::parse::Parser;
use syn_grammar::grammar;
use syn_grammar::testing::Testable;
use syn_grammar::types::SpannedValue;

#[test]
fn integer_widths() {
    mod inner {
        use super::*;
        grammar! {
            grammar ints {
                pub p_i8 -> i8 = v:i8
                pub p_i16 -> i16 = v:i16
                pub p_i64 -> i64 = v:i64
                pub p_i128 -> i128 = v:i128
                pub p_isize -> isize = v:isize
                pub p_u8 -> u8 = v:u8
                pub p_u16 -> u16 = v:u16
                pub p_u32 -> u32 = v:u32
                pub p_u64 -> u64 = v:u64
                pub p_u128 -> u128 = v:u128
                pub p_usize -> usize = v:usize
            }
        }
    }
    use inner::ints;

    ints::parse_p_i8
        .parse_str("-128")
        .test()
        .assert_success_is(-128i8);
    ints::parse_p_i16
        .parse_str("-32768")
        .test()
        .assert_success_is(-32768i16);
    ints::parse_p_i64
        .parse_str("-9223372036854775808")
        .test()
        .assert_success_is(i64::MIN);
    ints::parse_p_i128
        .parse_str("-170141183460469231731687303715884105728")
        .test()
        .assert_success_is(i128::MIN);
    ints::parse_p_isize
        .parse_str("-42")
        .test()
        .assert_success_is(-42isize);
    ints::parse_p_u8
        .parse_str("255")
        .test()
        .assert_success_is(255u8);
    ints::parse_p_u16
        .parse_str("65535")
        .test()
        .assert_success_is(65535u16);
    ints::parse_p_u32
        .parse_str("4294967295")
        .test()
        .assert_success_is(u32::MAX);
    ints::parse_p_u64
        .parse_str("18446744073709551615")
        .test()
        .assert_success_is(u64::MAX);
    ints::parse_p_u128
        .parse_str("340282366920938463463374607431768211455")
        .test()
        .assert_success_is(u128::MAX);
    ints::parse_p_usize
        .parse_str("7")
        .test()
        .assert_success_is(7usize);

    // Overflow must arrive as an error, not be silently truncated.
    ints::parse_p_u8.parse_str("256").test().assert_failure();
}

#[test]
fn char_and_bool() {
    mod inner {
        use super::*;
        grammar! {
            grammar prim {
                pub p_char -> char = v:char
                pub p_bool -> bool = v:bool
                pub p_f64 -> f64 = v:f64
            }
        }
    }
    use inner::prim;

    prim::parse_p_char
        .parse_str("'x'")
        .test()
        .assert_success_is('x');
    prim::parse_p_bool
        .parse_str("true")
        .test()
        .assert_success_is(true);
    prim::parse_p_bool
        .parse_str("false")
        .test()
        .assert_success_is(false);
    prim::parse_p_f64
        .parse_str("2.5")
        .test()
        .assert_success_is(2.5f64);
}

/// The `spanned_*` family: it returns `SpannedValue<T>` with `value` and `span`.
/// It was completely untested, although `syn_grammar::types` is re-exported
/// specifically for it.
#[test]
fn spanned_family_returns_value_and_span() {
    mod inner {
        use super::*;
        grammar! {
            grammar sp {
                pub s_char -> SpannedValue<char> = v:spanned_char
                pub s_bool -> SpannedValue<bool> = v:spanned_bool
                pub s_i8 -> SpannedValue<i8> = v:spanned_i8
                pub s_i16 -> SpannedValue<i16> = v:spanned_i16
                pub s_i32 -> SpannedValue<i32> = v:spanned_i32
                pub s_i64 -> SpannedValue<i64> = v:spanned_i64
                pub s_i128 -> SpannedValue<i128> = v:spanned_i128
                pub s_isize -> SpannedValue<isize> = v:spanned_isize
                pub s_u8 -> SpannedValue<u8> = v:spanned_u8
                pub s_u16 -> SpannedValue<u16> = v:spanned_u16
                pub s_u32 -> SpannedValue<u32> = v:spanned_u32
                pub s_u64 -> SpannedValue<u64> = v:spanned_u64
                pub s_u128 -> SpannedValue<u128> = v:spanned_u128
                pub s_usize -> SpannedValue<usize> = v:spanned_usize
                pub s_f32 -> SpannedValue<f32> = v:spanned_f32
                pub s_f64 -> SpannedValue<f64> = v:spanned_f64
            }
        }
    }
    use inner::sp;

    assert_eq!(
        sp::parse_s_char
            .parse_str("'q'")
            .test()
            .assert_success()
            .value,
        'q'
    );
    assert!(
        sp::parse_s_bool
            .parse_str("true")
            .test()
            .assert_success()
            .value
    );
    assert_eq!(
        sp::parse_s_i8.parse_str("-8").test().assert_success().value,
        -8i8
    );
    assert_eq!(
        sp::parse_s_i16
            .parse_str("-16")
            .test()
            .assert_success()
            .value,
        -16i16
    );
    assert_eq!(
        sp::parse_s_i32
            .parse_str("-32")
            .test()
            .assert_success()
            .value,
        -32i32
    );
    assert_eq!(
        sp::parse_s_i64
            .parse_str("-64")
            .test()
            .assert_success()
            .value,
        -64i64
    );
    assert_eq!(
        sp::parse_s_i128
            .parse_str("-128")
            .test()
            .assert_success()
            .value,
        -128i128
    );
    assert_eq!(
        sp::parse_s_isize
            .parse_str("-1")
            .test()
            .assert_success()
            .value,
        -1isize
    );
    assert_eq!(
        sp::parse_s_u8.parse_str("8").test().assert_success().value,
        8u8
    );
    assert_eq!(
        sp::parse_s_u16
            .parse_str("16")
            .test()
            .assert_success()
            .value,
        16u16
    );
    assert_eq!(
        sp::parse_s_u32
            .parse_str("32")
            .test()
            .assert_success()
            .value,
        32u32
    );
    assert_eq!(
        sp::parse_s_u64
            .parse_str("64")
            .test()
            .assert_success()
            .value,
        64u64
    );
    assert_eq!(
        sp::parse_s_u128
            .parse_str("128")
            .test()
            .assert_success()
            .value,
        128u128
    );
    assert_eq!(
        sp::parse_s_usize
            .parse_str("1")
            .test()
            .assert_success()
            .value,
        1usize
    );

    let f32_value: SpannedValue<f32> = sp::parse_s_f32.parse_str("1.5").test().assert_success();
    assert!((f32_value.value - 1.5f32).abs() < f32::EPSILON);
    let f64_value: SpannedValue<f64> = sp::parse_s_f64.parse_str("2.5").test().assert_success();
    assert!((f64_value.value - 2.5f64).abs() < f64::EPSILON);

    // The span must carry real position data - otherwise the whole family
    // would be pointless. Via `parse_str` proc-macro2 runs in fallback mode,
    // where line/column exist (inside a real procedural macro only from
    // Rust 1.88 on, see GOALS.md).
    let spanned_value = sp::parse_s_u32.parse_str("77").test().assert_success();
    assert_eq!(spanned_value.span.start().line, 1);
}

/// The token filters. `digit`, `hex_digit` and `oct_digit` return `syn::LitInt`,
/// not `syn::Ident` - the catalogue claimed the opposite for a long time.
#[test]
fn token_filter() {
    mod inner {
        use super::*;
        grammar! {
            grammar tf {
                pub p_alpha -> String = v:alpha -> { v.to_string() }
                pub p_alnum -> String = v:alphanumeric -> { v.to_string() }
                pub p_digit -> String = v:digit -> { v.base10_digits().to_string() }
                pub p_hex -> String = v:hex_digit -> { v.base10_digits().to_string() }
                pub p_oct -> String = v:oct_digit -> { v.base10_digits().to_string() }
            }
        }
    }
    use inner::tf;

    tf::parse_p_alpha
        .parse_str("abc")
        .test()
        .assert_success_is("abc".to_string());
    tf::parse_p_alnum
        .parse_str("a1b2")
        .test()
        .assert_success_is("a1b2".to_string());
    tf::parse_p_digit
        .parse_str("123")
        .test()
        .assert_success_is("123".to_string());
    tf::parse_p_hex
        .parse_str("42")
        .test()
        .assert_success_is("42".to_string());
    tf::parse_p_oct
        .parse_str("17")
        .test()
        .assert_success_is("17".to_string());

    // `alpha` must not let digits through.
    tf::parse_p_alpha
        .parse_str("a1")
        .test()
        .assert_failure_contains("expected an alphabetic identifier");
}

/// The `lit_*` family returns the raw syn tokens instead of evaluated values.
#[test]
fn literal_token() {
    mod inner {
        use super::*;
        grammar! {
            grammar lits {
                pub p_int -> String = v:lit_int -> { v.base10_digits().to_string() }
                pub p_char -> char = v:lit_char -> { v.value() }
                pub p_bool -> bool = v:lit_bool -> { v.value() }
                pub p_float -> f64 = v:lit_float -> { v.base10_parse().unwrap() }
                pub p_str -> String = v:lit_str -> { v.value() }
                pub p_byte -> u8 = v:any_byte -> { v.value() }
            }
        }
    }
    use inner::lits;

    lits::parse_p_int
        .parse_str("1u8")
        .test()
        .assert_success_is("1".to_string());
    lits::parse_p_char
        .parse_str("'z'")
        .test()
        .assert_success_is('z');
    lits::parse_p_bool
        .parse_str("true")
        .test()
        .assert_success_is(true);
    lits::parse_p_float
        .parse_str("3.5")
        .test()
        .assert_success_is(3.5f64);
    lits::parse_p_str
        .parse_str("\"hello\"")
        .test()
        .assert_success_is("hello".to_string());
    lits::parse_p_byte
        .parse_str("b'A'")
        .test()
        .assert_success_is(b'A');
}

/// The syn interop builtins. They are missing, of all places, from the README
/// table that exists precisely for them.
#[test]
fn syn_interop_builtins() {
    mod inner {
        use super::*;
        grammar! {
            grammar interop {
                pub p_vis -> String = v:visibility -> { quote::quote!(#v).to_string() }
                pub p_named -> String = v:named_field -> { v.ident.as_ref().unwrap().to_string() }
                pub p_unnamed -> String = v:unnamed_field -> { quote::quote!(#v).to_string() }
                pub p_stmts -> usize = v:statements -> { v.len() }
                pub p_generics -> usize = v:generics -> { v.params.len() }
                pub p_ret -> String = v:return_type -> { quote::quote!(#v).to_string() }
            }
        }
    }
    use inner::interop;

    interop::parse_p_vis
        .parse_str("pub")
        .test()
        .assert_success_is("pub".to_string());
    interop::parse_p_named
        .parse_str("name: i32")
        .test()
        .assert_success_is("name".to_string());
    interop::parse_p_unnamed
        .parse_str("i32")
        .test()
        .assert_success_is("i32".to_string());
    interop::parse_p_stmts
        .parse_str("let a = 1; let b = 2;")
        .test()
        .assert_success_is(2usize);
    interop::parse_p_generics
        .parse_str("<T, U>")
        .test()
        .assert_success_is(2usize);
    interop::parse_p_ret
        .parse_str("-> i32")
        .test()
        .assert_success_is("-> i32".to_string());
}

/// Since `201162a`, `any_ident` accepts keywords (`Ident::parse_any`) and is
/// thus no longer functionally identical to `ident`. That is a behaviour change
/// without a signature change - exactly the kind that regresses unnoticed
/// without a test.
#[test]
fn any_ident_accepts_keywords_ident_does_not() {
    mod inner {
        use super::*;
        grammar! {
            grammar ids {
                pub p_any -> String = v:any_ident -> { v.to_string() }
                pub p_plain -> String = v:ident -> { v.to_string() }
            }
        }
    }
    use inner::ids;

    ids::parse_p_any
        .parse_str("type")
        .test()
        .assert_success_is("type".to_string());
    ids::parse_p_any
        .parse_str("fn")
        .test()
        .assert_success_is("fn".to_string());
    ids::parse_p_any
        .parse_str("normal")
        .test()
        .assert_success_is("normal".to_string());

    ids::parse_p_plain
        .parse_str("normal")
        .test()
        .assert_success_is("normal".to_string());
    ids::parse_p_plain.parse_str("type").test().assert_failure();
}

/// The three builtins the review identified as real gaps.
///
/// `syn::Pat` was not reachable at all via the `syn::` path because it has no
/// `impl Parse`; `inner_attrs` was missing as the counterpart of `outer_attrs`;
/// `lit_byte` completes the naming scheme of the `lit_*` family.
#[test]
fn newly_added_builtins() {
    mod inner {
        use super::*;
        grammar! {
            grammar gaps {
                pub p_pat -> String = v:pat -> { quote::quote!(#v).to_string() }
                pub p_inner -> usize = v:inner_attrs -> { v.len() }
                pub p_byte -> u8 = v:lit_byte -> { v.value() }
            }
        }
    }
    use inner::gaps;

    // Simple binding pattern, tuple pattern and or-pattern.
    gaps::parse_p_pat
        .parse_str("x")
        .test()
        .assert_success_is("x".to_string());
    gaps::parse_p_pat
        .parse_str("(a, b)")
        .test()
        .assert_success_is("(a , b)".to_string());
    gaps::parse_p_pat
        .parse_str("Some(v)")
        .test()
        .assert_success_is("Some (v)".to_string());
    // Or-pattern - exactly the case why syn offers no `impl Parse`.
    gaps::parse_p_pat.parse_str("A | B").test().assert_success();

    gaps::parse_p_inner
        .parse_str("#![allow(dead_code)]")
        .test()
        .assert_success_is(1usize);
    gaps::parse_p_inner
        .parse_str("")
        .test()
        .assert_success_is(0usize);

    gaps::parse_p_byte
        .parse_str("b'Z'")
        .test()
        .assert_success_is(b'Z');
}
