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

grammar! {
    grammar enterprise_errors {
        pub rule root -> () = expr -> { () }

        rule expr -> ()
            = term "+" expr # "Addition" -> { () }
            | term # "Expression" -> { () }

        rule term -> ()
            = factor "*" term # "Multiplication" -> { () }
            | factor # "Term" -> { () }

        rule factor -> ()
            = "(" expr ")" # "Parenthesized Expression" -> { () }
            | "id" # "Identifier" -> { () }
            | fail("missing factor") -> { () }
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

#[test]
fn test_deep_error_with_label_and_fail() {
    // Attempting "id + (" which should fail because "(" is not followed by a valid expr
    // The error should ideally point to the missing expr after "(", but also mention "Parenthesized Expression"
    // or at least be at the right depth.
    let err = enterprise_errors::parse_root
        .parse_str("id + (")
        .test()
        .assert_failure();
    
    println!("Actual Error: {}", err);
    
    // We want the error to be specific.
    // In our current (old) system, it might say "unexpected end of input" or "expected factor"
    // In the new system, we want it to reflect the most specific failure at the deepest point.
}

#[test]
fn test_label_priority() {
    // "id + id id" -> fails at the second "id"
    // It could be an Addition where expr failed to match another term.
    let err = enterprise_errors::parse_root
        .parse_str("id + id id")
        .test()
        .assert_failure();

    println!("Actual Error: {}", err);
}

#[test]
fn test_fail_built_in_enterprise() {
    // This should trigger the `fail` in `factor` if other things don't match.
    // "id + -" -> "id" is a term, "+" is matched, then it expects an expr.
    // expr starts with term, term starts with factor.
    // factor tries "(" (fail), "id" (fail), then hits `fail("missing factor")`.
    let err = enterprise_errors::parse_root
        .parse_str("id + -")
        .test()
        .assert_failure();

    println!("Actual Error: {}", err);
    assert!(err.to_string().contains("missing factor"));
}
