use syn::parse::Parser;
use syn_grammar::grammar;
use syn_grammar::testing::Testable;

grammar! {
    grammar default_labels {
        pub main = ("a" | "b")
    }
}

grammar! {
    grammar explicit_labels {
        pub main
            = "a" # "Letter A"
            | "b" # "Letter B"
    }
}

grammar! {
    grammar deep_error {
        pub main
            = "a" "b" # "AB"
            | "c" # "C"
    }
}

grammar! {
    grammar group_labels {
        pub main
            = ("a" # "A" | "b" # "B")
    }
}

#[test]
fn test_default_labels() {
    default_labels::parse_main
        .parse_str("x")
        .test()
        .assert_failure_contains("expected one of: `a`, `b`");
}

#[test]
fn test_explicit_labels() {
    explicit_labels::parse_main
        .parse_str("x")
        .test()
        .assert_failure_contains("expected one of: `Letter A`, `Letter B`");
}

#[test]
fn test_deep_error_wins() {
    // Input "a x" matches first part of AB, fails at "b". This is deep.
    // So error should be "expected 'b'", NOT "expected one of: AB, C".
    deep_error::parse_main
        .parse_str("a x")
        .test()
        .assert_failure_contains("expected `b`")
        .assert_failure_not_contains("expected one of:");
}

#[test]
fn test_group_labels() {
    group_labels::parse_main
        .parse_str("x")
        .test()
        .assert_failure_contains("expected one of: `A`, `B`");
}
