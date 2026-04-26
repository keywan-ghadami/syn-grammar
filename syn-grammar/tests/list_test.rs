use syn::parse::Parser;
use syn_grammar::grammar;
use syn_grammar::testing::Testable;

// Top-level grammar definitions to avoid non-local macro warnings

grammar! {
    grammar list_test1 {
        pub main -> Vec<String> = items:separated(string, ",") -> {
            items.into_iter().map(|s| s.value).collect()
        }
    }
}

grammar! {
    grammar list_test2 {
        pub main -> Vec<String>
            = items:separated(string, ",", trailing=true) -> {
                items.into_iter().map(|s| s.value).collect()
            }

        pub strict -> Vec<String>
            = items:separated(string, ",") -> {
                items.into_iter().map(|s| s.value).collect()
            }
    }
}

grammar! {
    grammar list_test3 {
        pub min_two -> Vec<String>
            = items:separated(string, ",", min=2) -> {
                items.into_iter().map(|s| s.value).collect()
            }
    }
}

grammar! {
    grammar list_test4 {
        pub repeated_rule -> Vec<String>
            = items:repeated(string) -> {
                items.into_iter().map(|s| s.value).collect()
            }
    }
}

grammar! {
    grammar list_test5 {
        pub repeated_min -> Vec<String>
            = items:repeated(string, min=2) -> {
                items.into_iter().map(|s| s.value).collect()
            }
    }
}

grammar! {
    grammar list_test6 {
        // usage: separated<Vec>(...)
        pub explicit_container -> Vec<String>
            = items:separated<Vec>(string, ",") "end" -> {
                items.into_iter().map(|s| s.value).collect()
            }
    }
}

grammar! {
    grammar list_test_custom_error {
        pub main -> Vec<String>
            = items:separated(string, ",", item_label="function argument") -> {
                items.into_iter().map(|s| s.value).collect()
            }
    }
}

#[test]
fn test_separated_basic() {
    list_test1::parse_main
        .parse_str(r#""a", "b", "c""#)
        .test()
        .assert_success_is(vec!["a".to_string(), "b".to_string(), "c".to_string()]);

    list_test1::parse_main
        .parse_str(r#""a""#)
        .test()
        .assert_success_is(vec!["a".to_string()]);

    list_test1::parse_main
        .parse_str("")
        .test()
        .assert_success_is(Vec::<String>::new());
}

#[test]
fn test_separated_trailing() {
    list_test2::parse_main
        .parse_str(r#""a", "b","#)
        .test()
        .assert_success_is(vec!["a".to_string(), "b".to_string()]);

    list_test2::parse_strict
        .parse_str(r#""a", "b","#)
        .test()
        .assert_failure_contains("unexpected end of input, expected item")
        .assert_failure_contains("in strict");
}

#[test]
fn test_separated_min() {
    list_test3::parse_min_two
        .parse_str(r#""a""#)
        .test()
        .assert_failure_contains("expected at least 2 items, found 1 at column");

    list_test3::parse_min_two
        .parse_str(r#""a", "b""#)
        .test()
        .assert_success_is(vec!["a".to_string(), "b".to_string()]);
}

#[test]
fn test_repeated() {
    list_test4::parse_repeated_rule
        .parse_str(r#""a" "b" "c""#)
        .test()
        .assert_success_is(vec!["a".to_string(), "b".to_string(), "c".to_string()]);

    list_test4::parse_repeated_rule
        .parse_str("")
        .test()
        .assert_success_is(Vec::<String>::new());
}

#[test]
fn test_repeated_min() {
    list_test5::parse_repeated_min
        .parse_str(r#""a""#)
        .test()
        .assert_failure_contains("expected at least 2 items, found 1 at column");

    list_test5::parse_repeated_min
        .parse_str(r#""a" "b""#)
        .test()
        .assert_success_is(vec!["a".to_string(), "b".to_string()]);
}

#[test]
fn test_explicit_container() {
    list_test6::parse_explicit_container
        .parse_str(r#""a", "b" end"#)
        .test()
        .assert_success_is(vec!["a".to_string(), "b".to_string()]);
}

#[test]
fn test_separated_custom_error() {
    list_test_custom_error::parse_main
        .parse_str(r#""a", "b", end"#) // Trailing comma -> Error
        .test()
        .assert_failure_contains("expected function argument at column")
        .assert_failure_contains("in function argument 3")
        .assert_failure_contains("in main");
}
