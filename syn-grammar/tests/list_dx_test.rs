use syn::parse::Parser;
use syn_grammar::grammar;
use syn_grammar::testing::Testable;

grammar! {
    grammar cxx_dx {
        pub signature
            = "fn" ident paren( separated(param, "," , trailing=false, item_label="function parameter") ) ";" # "function signature"

        param
            = modifier? type_name ref_qual? ident # "function parameter"

        modifier
            = ("const" | "constexpr") # "type modifier"

        ref_qual
            = ("&" | "&&" | "*") # "reference qualifier"

        type_name 
            = ident ("::" ident)* # "type name"
    }
}

#[test]
fn test_cxx_shallow_wrong_token() {
    cxx_dx::parse_signature
        .parse_str("fn foo( 123 );")
        .test()
        .assert_failure_contains(
            "expected `function parameter`; found unexpected token `123` at column 8 (line 1)\nin param\nin function parameter 1\nin signature",
        );
}

#[test]
fn test_cxx_deep_type_error_after_modifier() {
    cxx_dx::parse_signature
        .parse_str("fn foo( const 123 );")
        .test()
        .assert_failure_contains(
            "expected `type name`; found unexpected token `123` at column 14 (line 1)\nin type name\nin param\nin function parameter 1\nin signature",
        );
}

#[test]
fn test_cxx_deep_ident_error_mid_list() {
    cxx_dx::parse_signature
        .parse_str("fn foo( int a, const std::string & 123 );")
        .test()
        .assert_failure_contains(
            "expected identifier at column 35 (line 1)\nin param\nin function parameter 2\nin signature",
        );
}

#[test]
fn test_cxx_dangling_comma() {
    cxx_dx::parse_signature
        .parse_str("fn foo( int a, );")
        .test()
        .assert_failure_contains(
            "unexpected end of input, expected function parameter at column 15 (line 1)\nin function parameter 2\nin signature",
        );
}

#[test]
fn test_cxx_unexpected_eof() {
    cxx_dx::parse_signature
        // Wir schließen die Klammer und den Ausdruck ab, um Syn's Lexer zu befriedigen.
        // Der Parser sollte beim Lesen von ":: )" fehlschlagen und "expected identifier" melden.
        .parse_str("fn foo( const std:: );")
        .test()
        .assert_failure_contains(
            "unexpected end of input, expected identifier at column 20 (line 1)\nin type name\nin param\nin function parameter 1\nin signature",
        );
}

#[test]
fn test_cxx_garbage_after_item() {
    cxx_dx::parse_signature
        .parse_str("fn foo( int a garbage );")
        .test()
        .assert_failure_contains(
            "expected `,` at column 14 (line 1)\nin separator\nin signature"
        );
}
