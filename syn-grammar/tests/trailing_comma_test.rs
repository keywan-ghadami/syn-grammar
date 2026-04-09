use syn::parse::Parser;
use syn_grammar::grammar;
use syn_grammar::testing::Testable;

grammar! {
    grammar trailing_comma_handling {
        // Helper parser that transforms a `LitStr` into a `String`.
        string_val -> String = s:string -> { s.value }

        // This parser demonstrates the correct behavior. `separated` should not fail on a
        // trailing comma, but leave it in the stream. The subsequent `","?` consumes it.
        pub consumes_comma -> Vec<String> =
            items:separated(string_val, ",")
            ","? // Explicitly consume the optional trailing comma.
            -> { items }

        // This parser verifies that `separated` does not consume the trailing comma.
        // Without a subsequent rule to consume the comma, a parse with a trailing comma
        // will fail due to the unexpected token.
        pub does_not_consume_comma -> Vec<String> =
            items:separated(string_val, ",") -> { items }
    }
}

#[test]
fn test_consumes_trailing_comma_succeeds() {
    // Validates that the grammar correctly parses a list with a trailing comma.
    trailing_comma_handling::parse_consumes_comma
        .parse_str(r#""a", "b","#)
        .test()
        .assert_success_is(vec!["a".to_string(), "b".to_string()]);
}

#[test]
fn test_does_not_consume_trailing_comma_fails() {
    // Proves that `separated` leaves the trailing comma in the stream, causing a failure
    // in a grammar that isn't designed to consume it.
    trailing_comma_handling::parse_does_not_consume_comma
        .parse_str(r#""a", "b","#)
        .test()
        .assert_failure_contains("unexpected token");
}

#[test]
fn test_no_trailing_comma_succeeds() {
    // Sanity check to ensure both parsers still work without a trailing comma.
    trailing_comma_handling::parse_consumes_comma
        .parse_str(r#""a", "b""#)
        .test()
        .assert_success_is(vec!["a".to_string(), "b".to_string()]);

    trailing_comma_handling::parse_does_not_consume_comma
        .parse_str(r#""a", "b""#)
        .test()
        .assert_success_is(vec!["a".to_string(), "b".to_string()]);
}
