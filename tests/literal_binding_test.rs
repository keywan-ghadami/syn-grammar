use syn::parse::Parser;
use syn_grammar::grammar;
use syn_grammar::testing::Testable;

#[test]
fn test_literal_binding() {
    grammar! {
        grammar LitBind1 {
            // The literal binding `a:"a"` binds `a` to a generated token struct (e.g. `kw::a`).
            // These tokens don't carry values, so we return hardcoded strings to verify the path.
            pub rule main -> (String, String) =
                a:"a" b:"b" -> { ("a".to_string(), "b".to_string()) }
        }
    }

    LitBind1::parse_main
        .parse_str("a b")
        .test()
        .assert_success_is(("a".to_string(), "b".to_string()));
}

#[test]
fn test_literal_binding_char() {
    grammar! {
        grammar LitBind2 {
            pub rule main -> (char, char) =
                a:'a' b:'b' -> { ('a', 'b') }
        }
    }

    LitBind2::parse_main
        .parse_str("a b")
        .test()
        .assert_success_is(('a', 'b'));
}

#[test]
fn test_literal_binding_raw_string() {
    grammar! {
        grammar LitBind3 {
            pub rule main -> (String, String) =
                a:r"a" b:r#"b"# -> { ("a".to_string(), "b".to_string()) }
        }
    }

    LitBind3::parse_main
        .parse_str("a b")
        .test()
        .assert_success_is(("a".to_string(), "b".to_string()));
}
