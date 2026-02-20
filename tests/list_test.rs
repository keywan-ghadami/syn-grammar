use syn::parse::Parser;
use syn_grammar::grammar;
use syn_grammar::testing::Testable;

#[test]
fn test_separated_basic() {
    grammar! {
        grammar list_test1 {
            pub rule main -> Vec<String> = items:separated(string, ",") -> {
                items.into_iter().map(|s| s.value).collect()
            }
        }
    }

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
    grammar! {
        grammar list_test2 {
            pub rule main -> Vec<String>
                = items:separated(string, ",", trailing=true) -> {
                    items.into_iter().map(|s| s.value).collect()
                }

            pub rule strict -> Vec<String>
                = items:separated(string, ",") -> {
                    items.into_iter().map(|s| s.value).collect()
                }
        }
    }

    list_test2::parse_main
        .parse_str(r#""a", "b","#)
        .test()
        .assert_success_is(vec!["a".to_string(), "b".to_string()]);

    list_test2::parse_strict
        .parse_str(r#""a", "b","#)
        .test()
        .assert_failure_contains("expected item after separator");
}

#[test]
fn test_separated_min() {
    grammar! {
        grammar list_test3 {
            pub rule min_two -> Vec<String>
                = items:separated(string, ",", min=2) -> {
                    items.into_iter().map(|s| s.value).collect()
                }
        }
    }

    list_test3::parse_min_two
        .parse_str(r#""a""#)
        .test()
        .assert_failure_contains("expected at least 2 items");

    list_test3::parse_min_two
        .parse_str(r#""a", "b""#)
        .test()
        .assert_success_is(vec!["a".to_string(), "b".to_string()]);
}

#[test]
fn test_repeated() {
    grammar! {
        grammar list_test4 {
            pub rule repeated_rule -> Vec<String>
                = items:repeated(string) -> {
                    items.into_iter().map(|s| s.value).collect()
                }
        }
    }

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
    grammar! {
        grammar list_test5 {
            pub rule repeated_min -> Vec<String>
                = items:repeated(string, min=2) -> {
                    items.into_iter().map(|s| s.value).collect()
                }
        }
    }

    list_test5::parse_repeated_min
        .parse_str(r#""a""#)
        .test()
        .assert_failure_contains("expected at least 2 items");

    list_test5::parse_repeated_min
        .parse_str(r#""a" "b""#)
        .test()
        .assert_success_is(vec!["a".to_string(), "b".to_string()]);
}

#[test]
fn test_explicit_container() {
    grammar! {
        grammar list_test6 {
            // usage: separated<Vec>(...)
            pub rule explicit_container -> Vec<String>
                = items:separated<Vec>(string, ",") -> {
                    items.into_iter().map(|s| s.value).collect()
                }
        }
    }

    list_test6::parse_explicit_container
        .parse_str(r#""a", "b""#)
        .test()
        .assert_success_is(vec!["a".to_string(), "b".to_string()]);
}
