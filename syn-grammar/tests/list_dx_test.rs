use syn::parse::Parser;
use syn_grammar::grammar;
use syn_grammar::testing::Testable;

grammar! {
    grammar cxx_dx {
        pub signature
            = "fn" ident paren( separated(param, ",") ) ";" # "function signature"

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
        .assert_failure_contains("expected type name at column")
        .assert_failure_contains("in function parameter 1")
        .assert_failure_contains("in function signature");
}

#[test]
fn test_cxx_deep_type_error_after_modifier() {
    cxx_dx::parse_signature
        .parse_str("fn foo( const 123 );")
        .test()
        .assert_failure_contains("expected type name at column")
        .assert_failure_contains("in function parameter 1")
        .assert_failure_contains("in function signature");
}

#[test]
fn test_cxx_deep_ident_error_mid_list() {
    cxx_dx::parse_signature
        .parse_str("fn foo( int a, const std::string & 123 );")
        .test()
        .assert_failure_contains("expected identifier at column")
        .assert_failure_contains("in function parameter 2")
        .assert_failure_contains("in function signature");
}

#[test]
fn test_cxx_dangling_comma() {
    cxx_dx::parse_signature
        .parse_str("fn foo( int a, );")
        .test()
        .assert_failure_contains("expected `)`")
        .assert_failure_contains("found unexpected token `,`")
        .assert_failure_contains("in function signature");
}

#[test]
fn test_cxx_unexpected_eof() {
    cxx_dx::parse_signature
        .parse_str("fn foo( const std::")
        .test()
        .assert_failure_contains("unexpected end of input at column")
        .assert_failure_contains("expected identifier")
        .assert_failure_contains("in type name")
        .assert_failure_contains("in function parameter 1");
}

#[test]
fn test_cxx_garbage_after_item() {
    cxx_dx::parse_signature
        .parse_str("fn foo( int a garbage );")
        .test()
        .assert_failure_contains("expected `)`")
        .assert_failure_contains("found unexpected token `garbage`")
        .assert_failure_contains("in function signature");
}
