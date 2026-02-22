use syn::parse::Parser;
use syn_grammar::grammar;
use syn_grammar::testing::Testable;

#[test]
fn test_epsilon_alternative() {
    grammar! {
        grammar epsilon {
            pub rule main -> Option<i32> =
                i:i32 -> { Some(i) }
              | -> { None }
        }
    }

    epsilon::parse_main
        .parse_str("42")
        .test()
        .assert_success_is(Some(42));
    epsilon::parse_main
        .parse_str("")
        .test()
        .assert_success_is(None);
}
