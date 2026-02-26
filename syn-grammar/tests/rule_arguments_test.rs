use syn::parse::Parser;
use syn_grammar::grammar;
use syn_grammar::testing::Testable;

#[test]
fn test_rule_arguments() {
    grammar! {
        grammar args {
            pub rule main -> i32 = "start" v:value(offset=10) -> { v }
            rule value(offset: i32) -> i32 = i:i32 -> { i + offset }
        }
    }

    args::parse_main
        .parse_str("start 5")
        .test()
        .assert_success_is(15);
}
