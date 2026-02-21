use syn_grammar::grammar;
use syn_grammar::testing::Testable;

#[test]
fn test_literal_binding() {
    mod inner {
        use super::*;
        grammar! {
            grammar G {
                pub rule main -> (i32, i32) =
                    a:"a" b:"b" -> { (a.parse().unwrap(), b.parse().unwrap()) }
            }
        }
    }

    inner::G::parse_main
        .parse_str("a b")
        .test()
        .assert_success_is((0, 0));
}

#[test]
fn test_literal_binding_char() {
    mod inner {
        use super::*;
        grammar! {
            grammar G {
                pub rule main -> (char, char) =
                    a:'a' b:'b' -> { (a.chars().next().unwrap(), b.chars().next().unwrap()) }
            }
        }
    }

    inner::G::parse_main
        .parse_str("a b")
        .test()
        .assert_success_is(('a', 'b'));
}

#[test]
fn test_literal_binding_raw_string() {
    mod inner {
        use super::*;
        grammar! {
            grammar G {
                pub rule main -> (String, String) =
                    a:r"a" b:r#"b"# -> { (a.to_string(), b.to_string()) }
            }
        }
    }

    inner::G::parse_main
        .parse_str("a b")
        .test()
        .assert_success_is(("a".to_string(), "b".to_string()));
}
