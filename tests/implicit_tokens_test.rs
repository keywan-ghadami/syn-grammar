use syn::parse::Parser;
use syn_grammar::grammar;
use syn_grammar::testing::Testable;

#[test]
fn test_implicit_char_literal() {
    grammar! {
        grammar char_lit_test {
            pub rule main -> () = '+' -> { () }
        }
    }

    char_lit_test::parse_main
        .parse_str("+")
        .test()
        .assert_success();
}

#[test]
fn test_implicit_token_aliases() {
    grammar! {
        grammar alias_test {
            pub rule plus -> () = PLUS -> { () }
            pub rule minus -> () = MINUS -> { () }
            pub rule dot -> () = DOT -> { () }
        }
    }

    alias_test::parse_plus
        .parse_str("+")
        .test()
        .assert_success();
    alias_test::parse_minus
        .parse_str("-")
        .test()
        .assert_success();
    alias_test::parse_dot.parse_str(".").test().assert_success();
}
