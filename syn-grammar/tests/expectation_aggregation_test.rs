//! ADR 13, point 6: `expected one of:` lists *every* alternative that failed
//! at its boundary - built-ins and rule calls included, not only the branches
//! whose first token can be peeked. Point 3: at the end of the scope the
//! enumeration says so.
//!
//! Before this, `factor` on `*` reported ``expected `Paren` `` - the `i32`
//! alternative was invisible and the delimiter carried its internal name.
use syn::parse::Parser;
use syn_grammar::grammar;
use syn_grammar::testing::Testable;

grammar! {
    grammar Calc {
        pub rule expression -> i32 = l:expression "+" r:term -> { l + r } | t:term -> { t }
        rule term -> i32 = f:factor "*" t:term -> { f * t } | f:factor -> { f }
        pub rule factor -> i32 = i:i32 -> { i } | paren(e:expression) -> { e }
        pub rule mixed -> i32 = "a" -> { 1 } | i:i32 -> { i } | s:string -> { 2 } | [ x:i32 ] -> { x }
        pub rule calls -> i32 = t:term -> { t } | s:string -> { 2 }
        pub rule labelled -> i32 = t:term # "a number" -> { t } | s:string -> { 2 }
        pub rule single -> i32 = i:i32 -> { i }
        pub rule grouped -> i32 = paren(f:factor) -> { f }
        pub rule filters -> () = a:alpha -> { let _ = a; } | d:digit -> { let _ = d; }

        // Every alternative starts with a rule call that matches the empty
        // input, so not one of them can be peeked.
        pub rule item -> () =
            outer_attrs "struct" ident ";" -> { () }
          | outer_attrs "enum" ident ";" -> { () }
          | "impl" ident ";" -> { () }
    }
}

/// A built-in alternative is listed next to a delimiter alternative.
#[test]
fn builtin_and_delimiter_are_both_listed() {
    Calc::parse_factor
        .parse_str("*")
        .test()
        .assert_failure_contains(
            "expected one of: `integer literal`, `parentheses`; found unexpected token `*`",
        );
}

/// The enumeration survives the way up through the rule stack.
#[test]
fn enumeration_keeps_the_rule_context() {
    Calc::parse_expression
        .parse_str("1 + *")
        .test()
        .assert_failure_contains("expected one of: `integer literal`, `parentheses`")
        .assert_failure_contains("\nin factor\nin term\nin expression");
}

/// Literals, built-ins and delimiters mix in one sorted list.
#[test]
fn all_alternative_kinds_are_listed() {
    Calc::parse_mixed
        .parse_str("*")
        .test()
        .assert_failure_contains(
            "expected one of: `a`, `integer literal`, `square brackets`, `string literal`",
        );
}

/// A rule call that failed at its boundary contributes the enumeration it
/// collected itself - the union goes through nested rules.
#[test]
fn nested_rule_expectations_are_unioned() {
    Calc::parse_calls
        .parse_str("*")
        .test()
        .assert_failure_contains(
            "expected one of: `integer literal`, `parentheses`, `string literal`",
        );
}

/// A label replaces what the branch would have listed on its own.
#[test]
fn label_replaces_the_inner_enumeration() {
    Calc::parse_labelled
        .parse_str("*")
        .test()
        .assert_failure_contains("expected one of: `a number`, `string literal`")
        .assert_failure_not_contains("integer literal");
}

/// A single built-in keeps its own wording - no backticks, no list.
#[test]
fn single_builtin_keeps_syn_wording() {
    Calc::parse_single
        .parse_str("*")
        .test()
        .assert_failure_contains("expected integer literal at column 0")
        .assert_failure_not_contains("expected `");
    Calc::parse_single
        .parse_str("")
        .test()
        .assert_failure_contains("unexpected end of input, expected integer literal");
}

/// At the end of the input or of a group the enumeration says so (point 3).
#[test]
fn enumeration_names_the_end_of_scope() {
    Calc::parse_factor
        .parse_str("")
        .test()
        .assert_failure_contains(
            "unexpected end of input, expected one of: `integer literal`, `parentheses`",
        );
    Calc::parse_grouped
        .parse_str("( )")
        .test()
        .assert_failure_contains(
            "unexpected end of group, expected one of: `integer literal`, `parentheses`",
        );
}

/// Token filters carry the expectation of the token they filter.
#[test]
fn token_filters_are_listed() {
    Calc::parse_filters
        .parse_str("*")
        .test()
        .assert_failure_contains("expected one of: `identifier`, `integer literal`");
}

/// An alternative whose first pattern can match nothing (`outer_attrs`, `x?`,
/// `x*`) is not peekable, so the branch is entered and fails inside. Its first
/// token is still what it would have accepted, and belongs in the enumeration
/// (point 6).
///
/// Before this, the two `outer_attrs` branches contributed nothing at all and
/// the message was the bare ``expected `impl` `` of the one peekable branch -
/// which named the only item kind that was *not* meant.
#[test]
fn alternatives_behind_a_nullable_prefix_are_listed() {
    Calc::parse_item
        .parse_str("42;")
        .test()
        .assert_failure_contains(
            "expected one of: `enum`, `impl`, `struct`; found unexpected token `42`",
        );
}
