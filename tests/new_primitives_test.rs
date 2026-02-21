use syn_grammar::grammar;
use syn_grammar::testing::Testable;

#[test]
fn test_float_primitive() {
    mod inner {
        use super::*;
        grammar! {
            grammar float_test {
                pub rule main -> f32 = f:FLOAT -> { f.parse().unwrap() };
            }
        }
    }

    inner::float_test::parse_main
        .parse_str("1.23")
        .test()
        .assert_success_is(1.23);
    inner::float_test::parse_main
        .parse_str("1.23e-5")
        .test()
        .assert_success_is(1.23e-5);
    inner::float_test::parse_main
        .parse_str("1e5")
        .test()
        .assert_success_is(1e5);
}

#[test]
fn test_numeric_primitives() {
    mod inner {
        use super::*;
        grammar! {
            grammar num_test {
                pub rule int -> i32 = i:INT -> { i.parse().unwrap() };
                pub rule hex -> i32 = h:HEX -> { i32::from_str_radix(h, 16).unwrap() };
                pub rule oct -> i32 = o:OCT -> { i32::from_str_radix(o, 8).unwrap() };
                pub rule bin -> i32 = b:BIN -> { i32::from_str_radix(b, 2).unwrap() };
            }
        }
    }

    inner::num_test::parse_int.parse_str("123").test().assert_success_is(123);
    inner::num_test::parse_hex.parse_str("ff").test().assert_success_is(255);
    inner::num_test::parse_oct.parse_str("77").test().assert_success_is(63);
    inner::num_test::parse_bin.parse_str("11").test().assert_success_is(3);
}

#[test]
fn test_string_primitive() {
    mod inner {
        use super::*;
        grammar! {
            grammar str_test {
                pub rule main -> String = s:STRING -> { s };
            }
        }
    }

    inner::str_test::parse_main
        .parse_str("\"hello\"")
        .test()
        .assert_success_is("hello".to_string());
    inner::str_test::parse_main
        .parse_str(r#""hello \"b\"""#)
        .test()
        .assert_success_is("hello \\\"b\\\"".to_string());
}

#[test]
fn test_whitespace_primitive() {
    mod inner {
        use super::*;
        grammar! {
            grammar ws_test {
                // Requires a non-whitespace token to be present after the ws
                pub rule main -> () = WS "a" -> {()};
            }
        }
    }

    inner::ws_test::parse_main.parse_str("a").test().assert_success_is(());
    inner::ws_test::parse_main.parse_str(" a").test().assert_success_is(());
    inner::ws_test::parse_main.parse_str("  a").test().assert_success_is(());
    inner::ws_test::parse_main.parse_str("\t a").test().assert_success_is(());
}

#[test]
fn test_whitespace_punct_ident() {
    mod inner {
        use super::*;
        grammar! {
            grammar ws_punct_ident_test {
                pub rule main -> () = "+=" "a" -> {()};
            }
        }
    }

    inner::ws_punct_ident_test::parse_main
        .parse_str("+= a")
        .test()
        .assert_success_is(());
}

#[test]
fn test_whitespace_ident_ident() {
    mod inner {
        use super::*;
        grammar! {
            grammar ws_ident_ident_test {
                pub rule main -> () = "a" "b" -> {()};
            }
        }
    }
    inner::ws_ident_ident_test::parse_main
        .parse_str("a b")
        .test()
        .assert_success_is(());
}

#[test]
fn test_whitespace_between_rules() {
    mod inner {
        use super::*;
        grammar! {
            grammar ws_between_rules_test {
                pub rule main -> () = a b -> {()};
                rule a -> () = "a" -> {()};
                rule b -> () = "b" -> {()};
            }
        }
    }
    inner::ws_between_rules_test::parse_main
        .parse_str("a b")
        .test()
        .assert_success_is(());
}
