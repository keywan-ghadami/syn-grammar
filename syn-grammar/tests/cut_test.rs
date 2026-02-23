use syn::parse::Parser;
use syn_grammar::grammar;
use syn_grammar::testing::Testable;

// --- Test Cut Operator ---
#[test]
fn test_cut_operator() {
    grammar! {
        grammar cut_test {
            pub rule main -> i32 =
                // If "let" is seen, commit to this arm.
                // If "let" is followed by something other than integer, fail immediately.
                "let" => i:i32 -> { i }
              | i:i32 -> { i }
        }
    }

    // "let 42" -> 42
    cut_test::parse_main
        .parse_str("let 42")
        .test()
        .assert_success_is(42);

    // "42" -> 42 (second arm)
    cut_test::parse_main
        .parse_str("42")
        .test()
        .assert_success_is(42);

    // "let bad" -> Error (expected integer), does NOT backtrack to second arm
    cut_test::parse_main
        .parse_str("let bad")
        .test()
        .assert_failure_contains("expected integer");
}

// --- Test Cut in Repetition ---
#[test]
fn test_cut_in_repetition() {
    grammar! {
        grammar cut_rep {
            // Helper rule to avoid binding a group directly, which is currently unsupported.
            // TODO: Implement direct group bindings (e.g. `items:("item" => i:i32)*`).
            pub rule main -> Vec<i32> =
                items:item_with_cut* -> { items }

            rule item_with_cut -> i32 =
                "item" => i:i32 -> { i }
        }
    }

    cut_rep::parse_main
        .parse_str("item 1 item 2")
        .test()
        .assert_success_is(vec![1, 2]);

    // "item 1 item" -> Fail (expected integer after second item)
    cut_rep::parse_main
        .parse_str("item 1 item")
        .test()
        .assert_failure_contains("expected integer");
}
