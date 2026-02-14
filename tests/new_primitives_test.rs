use syn::parse::Parser;
use syn_grammar::grammar;
use syn_grammar::testing::Testable;

// --- Test Float Primitive ---
#[test]
fn test_float_primitive() {
    grammar! {
        grammar float_test {
            rule main -> f64 = f:float -> { f }
        }
    }

    // Happy path
    float_test::parse_main
        .parse_str("3.14")
        .test()
        .assert_success();

    // Verify value
    let val = float_test::parse_main.parse_str("3.14").unwrap();
    assert!((val - 3.14).abs() < 1e-6);

    // Integers should fail (syn::LitFloat does not match integer literals unless they have . or exponent)
    float_test::parse_main
        .parse_str("42")
        .test()
        .assert_failure();
}

// --- Test Whitespace Primitive ---
#[test]
fn test_whitespace_primitive() {
    grammar! {
        grammar ws_test {
            // Require whitespace between "a" and "b"
            rule main -> () = "a" whitespace "b" -> { () }
        }
    }

    // "a b" -> OK (whitespace exists)
    ws_test::parse_main.parse_str("a b").test().assert_success();

    // "a   b" -> OK
    ws_test::parse_main
        .parse_str("a   b")
        .test()
        .assert_success();
}

#[test]
fn test_whitespace_punct_ident() {
    grammar! {
        grammar ws_punct {
            rule main -> () = "@" whitespace "detached" -> { () }
        }
    }

    // "@ detached" -> OK
    ws_punct::parse_main
        .parse_str("@ detached")
        .test()
        .assert_success();

    // "@detached" -> FAIL (adjacent)
    ws_punct::parse_main
        .parse_str("@detached")
        .test()
        .assert_failure_contains("expected whitespace");
}

#[test]
fn test_whitespace_ident_ident() {
    grammar! {
        grammar ws_ident {
            rule main -> () = "a" whitespace "b" -> { () }
        }
    }

    ws_ident::parse_main
        .parse_str("a b")
        .test()
        .assert_success();
}

#[test]
fn test_whitespace_between_rules() {
    grammar! {
        grammar ws_rules {
            rule main -> () = a whitespace b -> { () }
            rule a -> () = "a" -> { () }
            rule b -> () = "b" -> { () }
        }
    }

    // "a b" -> OK
    ws_rules::parse_main
        .parse_str("a b")
        .test()
        .assert_success();
}
