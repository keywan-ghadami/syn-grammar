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

grammar! {
    grammar rule_labels {
        // The label stands at the definition, so every call site inherits it -
        // instead of repeating `# "…"` at each of them.
        rule shared_struct # "a shared struct" -> () = outer_attrs "struct" ident ";" -> { () }
        rule shared_enum # "a shared enum" -> () = outer_attrs "enum" ident ";" -> { () }
        rule shared_union # "a shared union" -> () = outer_attrs "union" ident ";" -> { () }

        pub main -> () =
            s:shared_struct -> { s }
          | e:shared_enum   -> { e }
          // A label at the call site still wins over the rule's own.
          | u:shared_union # "an item, spelled out here" -> { u }
    }
}

/// A rule names itself once and every caller inherits it.
#[test]
fn rule_label_is_inherited_by_the_call_site() {
    rule_labels::parse_main
        .parse_str("42")
        .test()
        .assert_failure_contains("`a shared struct`")
        .assert_failure_contains("`a shared enum`");
}

/// The call site keeps the last word: its own label replaces the inherited one.
#[test]
fn call_site_label_beats_the_rule_label() {
    rule_labels::parse_main
        .parse_str("42")
        .test()
        .assert_failure_contains("`an item, spelled out here`");
}

/// A labelled rule is not additionally unfolded into its start tokens: the
/// author asked for a word, not for a token list.
#[test]
fn rule_label_replaces_the_grouped_form() {
    rule_labels::parse_main
        .parse_str("42")
        .test()
        .assert_failure_not_contains("shared struct(");
}
