use syn::parse::Parser;
use syn_grammar::grammar;
use syn_grammar::testing::Testable;

// These tests verify that our error handling abstractions (Cut, Fail) work as expected.
// Even if implemented via macros currently, they should adhere to the semantics defined in ADR-11.

grammar! {
    grammar cut_semantics {
        // If "commits" matches, we MUST NOT backtrack to "fallback".
        pub rule main -> ()
            = "commit" => "success" -> { () }
            | "fallback" -> { () }
    }
}

#[test]
fn test_cut_prevents_backtracking() {
    // "commit success" -> OK
    cut_semantics::parse_main
        .parse_str("commit success")
        .test()
        .assert_success();

    // "commit fail" -> Error (expected "success"), NOT "fallback" or "expected fallback"
    // The cut commits to the first branch.
    cut_semantics::parse_main
        .parse_str("commit fail")
        .test()
        .assert_failure_contains("expected `success`");
}

grammar! {
    grammar fail_semantics {
        pub rule main -> ()
            = "check" val:fail_on_zero -> { () }

        rule fail_on_zero -> ()
            = "zero" => fail("zero is not allowed") -> { () }
            | "one" -> { () }
    }
}

#[test]
fn test_fail_overrides_everything() {
    // "check one" -> OK
    fail_semantics::parse_main
        .parse_str("check one")
        .test()
        .assert_success();

    // "check zero" -> Error "zero is not allowed"
    // The fail() is inside a cut branch, so it should definitely be the reported error.
    fail_semantics::parse_main
        .parse_str("check zero")
        .test()
        .assert_failure_contains("zero is not allowed");
}

grammar! {
    grammar nested_cut {
        pub rule main -> ()
            = outer_cut -> { () }
            | "other" -> { () }

        rule outer_cut -> ()
            = "start" => inner_cut -> { () }

        rule inner_cut -> ()
            = "inner" => "end" -> { () }
    }
}

#[test]
fn test_nested_cut_propagation() {
    // "start inner end" -> OK
    nested_cut::parse_main
        .parse_str("start inner end")
        .test()
        .assert_success();

    // "start inner fail" -> Error "expected `end`"
    // Outer cut commits to outer_cut rule.
    // Inner cut commits to first branch of inner_cut.
    nested_cut::parse_main
        .parse_str("start inner fail")
        .test()
        .assert_failure_contains("expected `end`");

    // "start fail" -> Error "expected `inner`"
    // Outer cut commits to outer_cut. inner_cut fails at "inner".
    nested_cut::parse_main
        .parse_str("start fail")
        .test()
        .assert_failure_contains("expected `inner`");
}

grammar! {
    grammar priority_interaction {
        pub rule main -> ()
            = "a" "b" "c" -> { () }
            | "a" fail("hard fail") -> { () }
    }
}

#[test]
fn test_fail_vs_deep_error() {
    // "a b d" -> matches first branch "a", "b", fails at "c".
    // second branch matches "a", then "fail".
    // Logic: "fail" has priority 2. "expected c" has priority 0 (but depth 2).
    // Currently, our heuristic says:
    // 1. Fatality wins.
    // 2. Depth wins.
    // 3. Priority wins.

    // "a b d" ->
    // Branch 1 error at index 2 ("d"). Depth = 2.
    // Branch 2 error at index 1 ("b"). Depth = 1.
    // So "expected c" should win because it made more progress.

    priority_interaction::parse_main
        .parse_str("a b d")
        .test()
        .assert_failure_contains("expected `c`");

    // "a d" ->
    // Branch 1 error at index 1 ("d"). Depth = 1. (expected "b")
    // Branch 2 error at index 1 ("d"). Depth = 1. (hard fail)
    // Both fail at same position.
    // "hard fail" has priority 2. "expected b" has priority 0.
    // So "hard fail" should win.

    priority_interaction::parse_main
        .parse_str("a d")
        .test()
        .assert_failure_contains("hard fail");
}
