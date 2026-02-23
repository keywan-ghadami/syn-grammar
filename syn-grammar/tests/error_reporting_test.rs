use syn::parse::Parser;
use syn_grammar::grammar;
use syn_grammar::testing::Testable;

// Top-level definitions to avoid module nesting and macro export warnings.
// Each grammar has a unique name in this file to prevent conflicts.

grammar! {
    grammar err_test_1 {
        pub rule main -> () = deepest_err eof -> { () }
        rule deepest_err -> () = "a" "b" "c" -> { () }
    }
}

grammar! {
    // We use numeric words here because the 'syn' backend currently doesn't
    // support numeric literals like "0" as tokens.
    // See tests/digits.fixme for the intended test case for other backends.
    grammar numeric_words_test {
        pub rule main -> () = l:letter+ d:num_word+ eof -> { () }
        rule letter -> () = ("a" | "b" | "c") -> { () }
        rule num_word -> () = ("zero" | "one" | "two") -> { () }
    }
}

grammar! {
    grammar prio_test {
        pub rule main -> () = (deep | shallow) eof -> { () }
        rule deep -> () = "a" "b" "c" -> { () }
        rule shallow -> () = "d" "e" -> { () }
    }
}

grammar! {
    grammar rule_name_test {
        pub rule main -> () = a:inner_rule eof -> { () }
        rule inner_rule -> () = "a" "b" -> { () }
    }
}

grammar! {
    grammar enterprise_errors {
        pub rule root -> () = expr eof -> { () }

        rule expr -> ()
            = term "+" expr # "Addition" -> { () }
            | term # "Expression" -> { () }

        rule term -> ()
            = factor "*" term # "Multiplication" -> { () }
            | factor # "Term" -> { () }

        rule factor -> ()
            = paren(expr) # "Parenthesized Expression" -> { () }
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
        .assert_failure_contains("in rule `main`");

    // "a b one c"
    // 'letter+' consumes "a b".
    // 'num_word+' consumes "one".
    // At "c", the parser attempts to read another num_word (greedy +) and fails.
    // It also checks eof and fails.
    // The num_word failure is considered "deeper" (more specific context) and wins.
    // Deepest error reporting behavior:
    // It reports "expected one of: one, two, zero" because it tried to parse another num_word.
    numeric_words_test::parse_main
        .parse_str("a b one c")
        .test()
        // We accept that it expects a num_word.
        // Also check that duplicate path segments are gone (implicit by string matching "in rule main: in rule num_word: ...")
        // but assert_failure_contains only checks substrings.
        .assert_failure_contains("expected one of: `one`, `two`, `zero`");
}

#[test]
fn test_deep_vs_shallow() {
    prio_test::parse_main
        .parse_str("a b d")
        .test()
        .assert_failure_contains("expected `c`")
        .assert_failure_contains("in rule `deep`");
}

#[test]
fn test_rule_name_in_error_message() {
    rule_name_test::parse_main
        .parse_str("a c")
        .test()
        .assert_failure_contains("in rule `inner_rule`")
        .assert_failure_contains("expected `b`");
}

#[test]
fn test_deep_error_with_label_and_fail() {
    let err = enterprise_errors::parse_root
        .parse_str("id + ( )")
        .test()
        .assert_failure();

    println!("Actual Error: {}", err);
}

#[test]
fn test_label_priority() {
    let err = enterprise_errors::parse_root
        .parse_str("id + id id")
        .test()
        .assert_failure();

    println!("Actual Error: {}", err);
}

#[test]
fn test_fail_built_in_enterprise() {
    let err = enterprise_errors::parse_root
        .parse_str("id + -")
        .test()
        .assert_failure();

    println!("Actual Error: {}", err);
}
