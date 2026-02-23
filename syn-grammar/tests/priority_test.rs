use syn::parse::Parser;
use syn_grammar::grammar;
use syn_grammar::testing::Testable;

#[test]
fn test_backtracking_priority() {
    grammar! {
        grammar priority {
            pub rule main -> i32 =
                // Longest match first
                "a" "b" "c" -> { 3 }
              | "a" "b"     -> { 2 }
              | "a"         -> { 1 }
        }
    }

    priority::parse_main
        .parse_str("a b c")
        .test()
        .assert_success_is(3);

    priority::parse_main
        .parse_str("a b")
        .test()
        .assert_success_is(2);

    priority::parse_main
        .parse_str("a")
        .test()
        .assert_success_is(1);
}
