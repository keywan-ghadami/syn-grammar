//! What a `syn` parser wanted, in the grammar's words.
//!
//! syn describes a failed `Type::parse` by enumerating all sixteen tokens a
//! type may begin with. That is exhaustive, unusable, and says nothing the
//! position does not already say — and it contributed *nothing* to
//! `expected one of:`, so an alternative delegating to `syn` was invisible in
//! the enumeration (ADR 13, points 2 and 6).
//!
//! The rule is: no progress → the type names itself; progress → syn's own,
//! more specific message stays.

use syn_grammar::grammar;
use syn_grammar::SynTestExt;

grammar! {
    grammar Delegating {
        pub rule typed -> syn::Type = ":" t:syn::Type -> { t }
        pub rule builtin_type -> syn::Type = ":" t:rust_type -> { t }
        pub rule pattern -> syn::Pat = "let" p:pat -> { p }

        pub rule item -> () =
            u:syn::ItemUse -> { let _ = u; }
          | "impl" ident -> { () }
    }
}

/// The type names itself instead of listing every token it could have started
/// with.
#[test]
fn a_syn_type_names_itself() {
    Delegating::parse_typed
        .parse_test(": ")
        .assert_failure_contains("unexpected end of input, expected Rust type")
        .assert_failure_not_contains("`dyn`");
}

/// …and in the middle of the input the end-of-input prefix is absent, as it
/// should be. `,` cannot start a type at all, so `syn` fails without reading
/// anything.
#[test]
fn the_end_of_input_prefix_is_only_added_at_the_end() {
    Delegating::parse_typed
        .parse_test(": ,")
        .assert_failure_contains("expected Rust type")
        .assert_failure_not_contains("unexpected end of input");
}

/// **The important half.** If `syn` got somewhere, its message is the more
/// specific one and stays: after `dyn` it names the missing trait, which
/// `expected Rust type` would throw away.
#[test]
fn a_syn_error_with_progress_keeps_its_own_message() {
    Delegating::parse_typed
        .parse_test(": dyn")
        .assert_failure_contains("expected identifier")
        .assert_failure_not_contains("expected Rust type");
}

/// The built-ins that delegate to `syn` are named too, not only the types
/// written as a path.
#[test]
fn syn_builtins_are_named_as_well() {
    Delegating::parse_builtin_type
        .parse_test(": ")
        .assert_failure_contains("expected Rust type");
    Delegating::parse_pattern
        .parse_test("let ")
        .assert_failure_contains("expected pattern");
}

/// The point of recording the expectation: an alternative that delegates to
/// `syn` is listed in the enumeration like any other. Before this it was
/// simply missing, and the message named only the alternative that was *not*
/// meant.
#[test]
fn a_syn_alternative_appears_in_the_enumeration() {
    Delegating::parse_item
        .parse_test("42")
        .assert_failure_contains("expected one of:")
        .assert_failure_contains("`use statement`")
        .assert_failure_contains("`impl`");
}
