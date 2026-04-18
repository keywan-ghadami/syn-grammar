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
        .assert_failure("expected function parameter at column 8 (line 1)\nin function parameter 1\nin signature");
}

#[test]
fn test_cxx_deep_type_error_after_modifier() {
    cxx_dx::parse_signature
        .parse_str("fn foo( const 123 );")
        .test()
        .assert_failure("expected type name at column 14 (line 1)\nin function parameter 1\nin signature");
}

#[test]
fn test_cxx_deep_ident_error_mid_list() {
    cxx_dx::parse_signature
        .parse_str("fn foo( int a, const std::string & 123 );")
        .test()
        .assert_failure("expected identifier at column 35 (line 1)\nin function parameter 2\nin signature");
}

#[test]
fn test_cxx_dangling_comma() {
    cxx_dx::parse_signature
        .parse_str("fn foo( int a, );")
        .test()
        .assert_failure("expected function parameter at column 15 (line 1)\nin function parameter 2\nin signature");
}

#[test]
fn test_cxx_unexpected_eof() {
    cxx_dx::parse_signature
        .parse_str("fn foo( const std::")
        .test()
        .assert_failure("unexpected end of input, expected identifier at column 18 (line 1)\nin type name\nin param\nin function parameter 1\nin signature");
}

#[test]
fn test_cxx_garbage_after_item() {
    cxx_dx::parse_signature
        .parse_str("fn foo( int a garbage );")
        .test()
        .assert_failure("unexpected token in delimited group at column 14 (line 1)\nin signature");
}
