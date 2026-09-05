//! Error message tests for the acceptance benchmark.
//!
//! `GOALS.md` makes `cxx-parser` the yardstick: it "must work great and deliver
//! perfect error messages". Until now only the success case was checked
//! (`src/cxx_parser.rs`, `parse_complex_cxx_bridge`) - with bare `assert_eq!`,
//! without the shared test framework and without a single assertion about an
//! error message.
//!
//! This file closes the gap. It checks the contract from
//! `docs/adr/adr13-error-message-contract.md` against real, broken CXX bridge
//! inputs - not against artificial minimal grammars.

use cxx_parser::CxxParser;
use syn::parse::Parser;
use syn_grammar::testing::Testable;

/// Shorthand: parse source text and lift it into the shared `TestResult`.
fn parse(src: &str) -> syn_grammar::testing::TestResult<cxx_parser::FfiMod, syn::Error> {
    CxxParser::parse_top_level_mod
        .parse_str(src)
        .test()
        .with_source(src)
}

/// ADR 13, point 4: the rule stack is printed on multiple lines from inside
/// out, de-snake-cased. That is the property that makes a message in a nested
/// grammar locatable in the first place.
#[test]
fn rule_stack_shows_the_way_from_inside_out() {
    parse(r#"mod ffi { extern "C++" { fn f(a: ); } }"#)
        .assert_failure_contains("in cxx arg")
        .assert_failure_contains("in cxx item")
        .assert_failure_contains("in extern block")
        .assert_failure_contains("in top level mod");
}

/// ADR 13, point 11: lists name their item and the index. The label comes
/// from `item_label="function argument"` in the grammar.
#[test]
fn list_error_names_item_and_index() {
    parse(r#"mod ffi { extern "C++" { fn f(a: i32, , b: i32); } }"#)
        .assert_failure_contains("expected function argument")
        .assert_failure_contains("in function argument 2");
}

/// The index counts along: the same error in the first argument names index 1.
#[test]
fn list_index_counts_along() {
    parse(r#"mod ffi { extern "C++" { fn f(a: ); } }"#)
        .assert_failure_contains("in function argument 1");
}

/// ADR 13, point 3: at the end of the input the prefix form instead of a bare
/// "expected" message.
#[test]
fn end_of_input_is_named() {
    parse(r#"mod ffi { extern "C++" { fn f(a: ); } }"#)
        .assert_failure_contains("unexpected end of input");
}

/// ADR 13, point 3: what was actually found is named - here a keyword that
/// is out of the question as an item name.
#[test]
fn found_token_is_named() {
    parse(r#"mod ffi { extern "C++" { struct S; } }"#)
        .assert_failure_contains("expected identifier")
        .assert_failure_contains("found keyword `struct`");
}

/// If the outermost rule fails immediately, the stack stays a single line - no
/// context is invented that does not exist.
#[test]
fn outermost_error_stays_brief() {
    parse(r#"extern "C++" { }"#)
        .assert_failure_contains("expected `mod`")
        .assert_failure_contains("in top level mod")
        .assert_failure_not_contains("in extern block");
}

/// A `syn` type parsed via the bridge returns its own message - and still gets
/// the grammar's rule context appended.
#[test]
fn syn_type_error_keeps_grammar_context() {
    parse(r#"mod ffi { extern C++ { } }"#)
        .assert_failure_contains("expected string literal")
        .assert_failure_contains("in extern block");
}

/// An argument that fails right at its start position is reported as a missing
/// list item - not as a missing separator.
///
/// `cxx_arg` fails on `123` without consuming a token. The list is optional
/// (`cxx_arg_list?`, i.e. `min=0`), so the reason is only recorded in
/// `ParseContext::furthest`. Right after that the optional `","?` fails at the
/// same position and is recorded as well.
///
/// Both are at the same cursor, and on a tie `merge` prefers the later one. So
/// that the meaningless separator error does not win here, the item expectation
/// gets the rank of a label (`PRIO_LABELED`); a mere token error has
/// `PRIO_NORMAL`. Previously ``expected `,` `` stood here.
#[test]
fn invalid_argument_is_reported_as_missing_item() {
    parse(r#"mod ffi { extern "C++" { fn f( 123 ); } }"#)
        .assert_failure_contains("expected function argument")
        .assert_failure_contains("in function argument 1")
        .assert_failure_contains("in cxx item")
        .assert_failure_contains("in extern block");
}
