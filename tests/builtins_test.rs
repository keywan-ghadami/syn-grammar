use syn::parse::Parser;
use syn_grammar::grammar;
use syn_grammar::testing::Testable;

#[test]
fn test_builtins() {
    grammar! {
        grammar builtins_test {
            pub rule test_int -> i32 = i:i32 -> { i }
            pub rule test_str -> String = s:string -> { s.value }
            pub rule test_ident -> String = i:ident -> { i.to_string() }
            pub rule test_eof -> () = eof -> { () }
        }
    }

    // --- Positive Tests ---

    // Integer: Standard
    builtins_test::parse_test_int
        .parse_str("123")
        .test()
        .assert_success_is(123);

    // Integer: Zero
    builtins_test::parse_test_int
        .parse_str("0")
        .test()
        .assert_success_is(0);

    // String: Standard
    builtins_test::parse_test_str
        .parse_str("\"hello\"")
        .test()
        .assert_success_is("hello".to_string());

    // String: Number inside string
    builtins_test::parse_test_str
        .parse_str("\"123\"")
        .test()
        .assert_success_is("123".to_string());

    // Ident: Standard
    builtins_test::parse_test_ident
        .parse_str("abc")
        .test()
        .assert_success_is("abc".to_string());

    // Ident: Underscore
    builtins_test::parse_test_ident
        .parse_str("_val")
        .test()
        .assert_success_is("_val".to_string());

    // EOF
    builtins_test::parse_test_eof
        .parse_str("")
        .test()
        .assert_success_is(());

    // Integer: Negative number (minus is separate token)
    builtins_test::parse_test_int
        .parse_str("-123")
        .test()
        .assert_success();

    // --- Integer Prefix/Suffix Tests ---
    // Note: These currently succeed because the underlying parser (likely syn::LitInt)
    // supports them. This behavior should be documented and potentially configurable
    // if we want strict base-10 or cross-backend consistency.

    // Integer: Octal prefix
    builtins_test::parse_test_int
        .parse_str("0o123")
        .test()
        .assert_success_is(83);

    // Integer: Hex prefix
    builtins_test::parse_test_int
        .parse_str("0x123")
        .test()
        .assert_success_is(0x123);

    // Integer: Binary prefix
    builtins_test::parse_test_int
        .parse_str("0b1010")
        .test()
        .assert_success_is(10);

    // Integer: Suffix (should probably fail if we want strict i32, but syn::LitInt might ignore it or we might be using it in a way that allows it)
    // TODO: Verify if suffixes should be allowed or if they should cause a failure for 'i32' builtin.
    builtins_test::parse_test_int
        .parse_str("123i32")
        .test()
        .assert_success_is(123);

    // --- Failure Tests ---

    // Integer: String literal -> Fail
    builtins_test::parse_test_int
        .parse_str("\"hello\"")
        .test()
        .assert_failure();

    // Integer: Ident -> Fail
    builtins_test::parse_test_int
        .parse_str("abc")
        .test()
        .assert_failure();

    // String: Integer literal -> Fail
    builtins_test::parse_test_str
        .parse_str("123")
        .test()
        .assert_failure();

    // String: Ident -> Fail
    builtins_test::parse_test_str
        .parse_str("abc")
        .test()
        .assert_failure();

    // Ident: Integer -> Fail
    builtins_test::parse_test_ident
        .parse_str("123")
        .test()
        .assert_failure();

    // Ident: String -> Fail
    builtins_test::parse_test_ident
        .parse_str("\"abc\"")
        .test()
        .assert_failure();
}
