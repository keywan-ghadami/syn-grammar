use syn::parse::Parser;
use syn_grammar::grammar;
use syn_grammar::testing::Testable;

#[test]
fn test_optional_production() {
    grammar! {
        grammar optional_prod {
            pub rule main -> i32 =
                i:inner? -> { i.unwrap_or(0) }

            rule inner -> i32 = "a" -> { 1 }
        }
    }

    optional_prod::parse_main
        .parse_str("a")
        .test()
        .assert_success_is(1);

    optional_prod::parse_main
        .parse_str("")
        .test()
        .assert_success_is(0);
}
