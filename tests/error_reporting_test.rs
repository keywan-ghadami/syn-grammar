use syn::parse::Parser;
use syn_grammar::grammar;
use syn_grammar::testing::Testable;

#[test]
fn test_deepest_error_wins() {
    {
        grammar! {
            grammar error_test {
                rule main -> String =
                    "a" "b" "c" -> { "abc".to_string() }
                  | "a" "b" "d" -> { "abd".to_string() }
            }
        }

        error_test::parse_main
            .parse_str("a b x")
            .test()
            .assert_failure_contains("expected 'c', 'd'");
    }

    {
        grammar! {
            grammar distinct {
                rule main -> String =
                    l:long -> { l }
                  | s:short -> { s }

                rule long -> String = "a" "b" "c" -> { "long".to_string() }
                rule short -> String = "a" "b" "d" -> { "short".to_string() }
            }
        }

        // Both paths fail at 'x'. 'long' fails at 3rd token. 'short' fails at 3rd token.
        // It should report both failures if they are at the same depth.
        distinct::parse_main
            .parse_str("a b x")
            .test()
            .assert_failure_contains("expected 'c', 'd'");
    }
}

#[test]
fn test_deep_vs_shallow() {
    grammar! {
        grammar priority {
            rule main -> String =
                d:deep -> { d }
              | s:shallow -> { s }

            rule deep -> String = "a" "b" "c" -> { "deep".to_string() }
            rule shallow -> String = "a" "x" -> { "shallow".to_string() }
        }
    }

    // Input "a b x".
    // 'deep' fails at 'c' (depth 2).
    // 'shallow' fails at 'x' (depth 1). expected 'x' but got 'b'? No.
    // shallow expects "a" then "x". Input "a" "b".
    // shallow matches "a". Then fails at "x" vs "b". Depth 1.
    // deep matches "a", "b". Then fails at "c" vs "x". Depth 2.
    // Deep error should win.
    priority::parse_main
        .parse_str("a b x")
        .test()
        .assert_failure_contains("expected 'c'");
}

#[test]
fn test_rule_name_in_error_message() {
    grammar! {
        grammar rule_context {
            rule main -> String =
                a:inner -> { a }

            rule inner -> String =
                "start" "end" -> { "ok".to_string() }
        }
    }

    // "start x". Inner fails at "end".
    // We expect standard "expected 'end'".
    // If we implemented context tracking, maybe "in rule inner".
    // Current impl just reports tokens.
    rule_context::parse_main
        .parse_str("start x")
        .test()
        .assert_failure_contains("expected 'end'");
}
