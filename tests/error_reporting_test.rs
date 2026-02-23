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
    // We use numeric words here because the 'syn' backend currently doesn't 
    // support numeric literals like "0" as tokens.
    // See tests/digits.fixme for the intended test case for other backends.
    grammar numeric_words_test {
        pub rule main -> () = l:letter+ d:num_word+ -> { () }
        rule letter -> () = ("a" | "b" | "c") -> { () }
        rule num_word -> () = ("zero" | "one" | "two") -> { () }
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
        .assert_failure_contains("expected `c`")
        .assert_failure_contains("in rule `deepest_err`")
        .assert_failure_contains("in rule `main`")
        ;

    numeric_words_test::parse_main
        .parse_str("a b one c")
        .test()
        .assert_failure_contains("expected one of: \"zero\", \"one\", \"two\"");
}

#[test]
fn test_deep_vs_shallow() {
    prio_test::parse_main
        .parse_str("a b d")
        .test()
        .assert_failure_contains("expected `c`")
        .assert_failure_contains("in rule `deep`")
        ;
}

#[test]
fn test_rule_name_in_error_message() {
    rule_name_test::parse_main
        .parse_str("a c")
        .test()
        .assert_failure_contains("in rule `inner_rule`")
        .assert_failure_contains("expected `b`")
        ;
}
