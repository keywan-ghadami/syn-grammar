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

grammar! {
    // The same list, but with `min=1` - that is the HARD path in
    // `parse_separated` (mandatory item, error is passed upward) instead of
    // the soft one (empty list allowed, error only recorded). Both must treat
    // the message of a failed item the same way.
    grammar cxx_min1 {
        pub signature
            = "fn" ident paren( separated(param, ",", min=1, item_label="function parameter") ) ";" # "function signature"
        param = modifier? ident # "function parameter"
        modifier = ("const" | "constexpr") # "type modifier"
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
            "unexpected end of group, expected function parameter at column 15 (line 1)\nin function parameter 2\nin signature",
        );
}

#[test]
fn test_cxx_unexpected_eof() {
    cxx_dx::parse_signature
        // We close the parenthesis and the expression to satisfy syn's lexer.
        // The parser should fail when reading ":: )" and report "expected identifier".
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
        .assert_failure_contains("expected `,` at column 14 (line 1)\nin separator\nin signature");
}

/// An item that fails right at its start position may keep its **own** label -
/// `finish_variants` additionally names in it what was actually there. The
/// poorer version `expected function parameter` would lose the
/// `; found unexpected token `123``.
///
/// Applies to the hard path (`min=1`) just as to the soft one; that was not the
/// case before.
#[test]
fn labelled_item_keeps_its_message_even_with_min1() {
    cxx_min1::parse_signature
        .parse_str("fn foo( 123 );")
        .test()
        .assert_failure_contains(
            "expected `function parameter`; found unexpected token `123` at column 8 (line 1)\nin param\nin function parameter 1\nin signature",
        );
}

/// At the end of the group the exception applies: there the item expectation
/// replaces even a labelled inner message. An enumeration of what could have
/// been there would be misleading - simply nothing follows any more
/// (ADR 13, point 3).
#[test]
fn at_group_end_item_expectation_wins() {
    cxx_min1::parse_signature
        .parse_str("fn foo( );")
        .test()
        .assert_failure_contains(
            "unexpected end of group, expected function parameter at column 8 (line 1)\nin function parameter 1\nin signature",
        );
}
