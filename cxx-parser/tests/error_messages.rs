//! Error message tests for the acceptance benchmark.
//!
//! `GOALS.md` makes `cxx-parser` the yardstick: it "must work great and deliver
//! perfect error messages". So the contract from
//! `docs/adr/adr13-error-message-contract.md` is checked here against real,
//! broken CXX bridge inputs — not against artificial minimal grammars.

use cxx_parser::{CxxParser, FfiMod};
use syn_grammar::testing::TestResult;
use syn_grammar::SynTestExt;

fn parse(src: &str) -> TestResult<FfiMod, syn::Error> {
    CxxParser::parse_top_level_mod.parse_test(src)
}

/// ADR 13, point 4: the rule stack is printed on multiple lines from inside
/// out, de-snake-cased. That is the property that makes a message in a nested
/// grammar locatable in the first place.
#[test]
fn rule_stack_shows_the_way_from_inside_out() {
    parse(r#"mod ffi { extern "C++" { fn f(a: ); } }"#)
        .assert_failure_contains("in param")
        .assert_failure_contains("in foreign fn")
        .assert_failure_contains("in extern item")
        .assert_failure_contains("in extern block")
        .assert_failure_contains("in mod item")
        .assert_failure_contains("in top level mod");
}

/// ADR 13, point 11: lists name their item and count it. The label comes from
/// `item_label="function parameter"` in the grammar.
#[test]
fn list_error_names_item_and_index() {
    parse(r#"mod ffi { extern "C++" { fn f(a: i32, , b: i32); } }"#)
        .assert_failure_contains("in function parameter 2");
}

/// The index counts along: the same error in the first parameter names index 1.
#[test]
fn list_index_counts_along() {
    parse(r#"mod ffi { extern "C++" { fn f( 123 ); } }"#)
        .assert_failure_contains("in function parameter 1");
}

/// ADR 13, point 11: a missing separator is reported as one, not as a broken
/// item.
#[test]
fn missing_separator_is_named_as_such() {
    parse(r#"mod ffi { extern "C++" { fn f(a: i32 b: i32); } }"#)
        .assert_failure_contains("expected `,`")
        .assert_failure_contains("in separator");
}

/// Lists in the shared types carry their own labels.
#[test]
fn struct_and_enum_lists_have_their_own_labels() {
    parse("mod ffi { struct S { a: u8, , } }").assert_failure_contains("in struct field 2");
    parse("mod ffi { enum E { A, , } }").assert_failure_contains("in enum variant 2");
}

/// ADR 13, point 3: at the end of the input the prefix form instead of a bare
/// "expected" message.
#[test]
fn end_of_input_is_named() {
    parse(r#"mod ffi { extern "C++" { fn f(a: ); } }"#)
        .assert_failure_contains("unexpected end of input");
}

/// ADR 13, point 3: what was actually found is named.
#[test]
fn found_token_is_named() {
    parse(r#"mod ffi { extern "C++" { fn f( 123 ); } }"#)
        .assert_failure_contains("found unexpected token `123`");
}

/// ADR 13, point 6: **every** alternative that failed at its boundary is
/// listed, not only those whose first token can be peeked. All five items of
/// the bridge module begin with `outer_attrs`, which matches the empty input,
/// so none of them is peekable.
#[test]
fn all_alternatives_are_listed_even_when_none_can_be_peeked() {
    parse("mod ffi { 42 }")
        .assert_failure_contains("expected one of:")
        .assert_failure_contains("a use statement")
        .assert_failure_contains("a shared struct")
        .assert_failure_contains("a shared enum")
        .assert_failure_contains("an extern block")
        .assert_failure_contains("an impl block");
}

/// The same inside an extern block: an item that does not exist in the bridge
/// language is answered with the three that do.
#[test]
fn unknown_extern_item_lists_the_known_ones() {
    parse(r#"mod ffi { extern "C++" { struct S; } }"#)
        .assert_failure_contains("expected one of:")
        .assert_failure_contains("an include")
        .assert_failure_contains("a type declaration")
        .assert_failure_contains("a function")
        .assert_failure_contains("found unexpected token `struct`");
}

/// ADR 13, point 7: an alternative that got further displaces the enumeration.
/// `unsafe` can only continue as a function, and that is what the message says
/// instead of listing all three item kinds again.
#[test]
fn depth_beats_aggregation() {
    parse(r#"mod ffi { extern "C++" { unsafe type Foo; } }"#)
        .assert_failure_contains("expected `fn`")
        .assert_failure_not_contains("expected one of");
}

/// The cut (`=>`) after `type` commits to the type item: the error names the
/// missing identifier instead of backtracking into an enumeration.
#[test]
fn cut_keeps_the_error_inside_the_committed_alternative() {
    parse(r#"mod ffi { extern "C++" { type 42; } }"#)
        .assert_failure_contains("expected identifier")
        .assert_failure_contains("in type item")
        .assert_failure_not_contains("expected one of");
}

/// If the outermost rule fails immediately, the stack stays a single line — no
/// context is invented that does not exist.
#[test]
fn outermost_error_stays_brief() {
    parse(r#"extern "C++" { }"#)
        .assert_failure_contains("expected `mod`")
        .assert_failure_contains("in top level mod")
        .assert_failure_not_contains("in extern block");
}

/// A `syn` type parsed via the bridge returns its own message — and still gets
/// the grammar's rule context appended.
#[test]
fn syn_error_keeps_grammar_context() {
    parse("mod ffi { extern C++ { } }")
        .assert_failure_contains("expected string literal")
        .assert_failure_contains("in extern block");
}

/// A hand-written `extern rule` (`extern_lang`) can say what the grammar
/// cannot express — the *content* of a string literal — and its message is
/// carried out with position and rule stack like any other.
#[test]
fn hand_written_rule_reports_the_language_it_wanted() {
    parse(r#"mod ffi { extern "Java" { } }"#)
        .assert_failure_contains(r#"expected "C++" or "Rust", found "Java""#)
        .assert_failure_contains("in extern block")
        .assert_failure_contains("in top level mod");
}

/// `fail(…)` states a rule the grammar cannot express structurally: cxx takes
/// the instantiation, never a body. Without it the message would be the
/// truthful but unhelpful `expected end of input`.
#[test]
fn fail_explains_a_rule_the_grammar_cannot_express() {
    parse("mod ffi { impl UniquePtr<T> { fn x(); } }")
        .assert_failure_contains("an `impl` in a bridge declares an instantiation and stays empty")
        .assert_failure_contains("in impl block");
}

/// `include!` is matched by name, so the wrong macro is answered with the right
/// item — not with a parse error about macro syntax.
#[test]
fn a_foreign_macro_is_answered_with_the_expected_items() {
    parse(r#"mod ffi { extern "C++" { printn!("x.h"); } }"#)
        .assert_failure_contains("an include")
        .assert_failure_contains("found unexpected token `printn`");
}

/// ADR 13, point 5: one position per message, and it points at the offending
/// token rather than at the start of the enclosing rule.
#[test]
fn the_position_points_at_the_offending_token() {
    let err = parse(r#"mod ffi { extern "C++" { fn f( 123 ); } }"#)
        .assert_failure()
        .to_string();
    assert_eq!(
        err.matches("at column").count(),
        1,
        "exactly one position per message:\n{err}"
    );
    assert!(err.contains("at column 31"), "column of `123`:\n{err}");
}
