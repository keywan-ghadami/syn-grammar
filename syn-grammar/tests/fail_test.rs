use syn::parse::Parser;
use syn_grammar::grammar;
use syn_grammar::testing::Testable;

#[test]
fn test_fail_builtin() {
    grammar! {
        grammar Fail {
            // Updated example: Use cut `=>` to ensure `fail` is fatal.
            // Also consume trailing identifiers to allow "foo baz".
            pub rule check -> () =
                "foo"
                (
                    "bar" => fail("foo cannot be followed by bar")
                  | -> { () }
                )
                ident*
                -> { () }
        }
    }

    // Success-Fall: "foo" followed by "baz".
    // "baz" is consumed by `ident*`, so the parse succeeds fully.
    Fail::parse_check
        .parse_str("foo baz")
        .test()
        .assert_success();

    // Success-Fall: "foo" followed by nothing.
    // `ident*` matches zero times.
    Fail::parse_check.parse_str("foo").test().assert_success();

    // Failure-Fall: "foo bar" triggers the explicit fail.
    // The `=>` prevents backtracking to the empty alternative.
    Fail::parse_check
        .parse_str("foo bar")
        .test()
        .assert_failure_contains("foo cannot be followed by bar");
}
