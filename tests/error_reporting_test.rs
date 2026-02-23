use syn::parse::Parser;
use syn_grammar::grammar;
use syn_grammar::testing::Testable;

// Top-level definitions to avoid module nesting and macro export warnings.
// Each grammar has a unique name in this file to prevent conflicts.

grammar! {
    grammar err_test_1 {
        pub rule main -> () = deepest_err -> { () }
        rule deepest_err -> () = "a" "b" "c" -> { () }
    }
}

grammar! {
    grammar digit_test_1 {
        pub rule main -> () = l:letter+ d:digit_rule+ -> { () }
        rule letter -> () = ("a" | "b" | "c") -> { () }
        rule digit_rule -> () = ("0" | "1" | "2") -> { () }
    }
}

grammar! {
    grammar prio_test {
        pub rule main -> () = (deep | shallow) -> { () }
        rule deep -> () = "a" "b" "c" -> { () }
        rule shallow -> () = "d" "e" -> { () }
    }
}

grammar! {
    grammar rule_name_test {
        pub rule main -> () = a:inner_rule -> { () }
        rule inner_rule -> () = "a" "b" -> { () }
    }
}

#[test]
fn test_deepest_error_wins() {
    err_test_1::parse_main
        .parse_str("a b d")
        .test()
        .assert_failure_contains("expected `c`");

    digit_test_1::parse_main
        .parse_str("a b 1 c")
        .test()
        .assert_failure_contains("expected one of: \"0\", \"1\", \"2\"");
}

#[test]
fn test_deep_vs_shallow() {
    prio_test::parse_main
        .parse_str("a b d")
        .test()
        .assert_failure_contains("expected `c`");
}

#[test]
fn test_rule_name_in_error_message() {
    rule_name_test::parse_main
        .parse_str("a c")
        .test()
        .assert_failure_contains("in rule `inner_rule`")
        .assert_failure_contains("expected `b`");
}
