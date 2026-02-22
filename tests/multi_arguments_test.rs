use syn::parse::Parser;
use syn_grammar::grammar;
use syn_grammar::testing::Testable;

#[test]
fn test_multiple_arguments() {
    grammar! {
        grammar multi_args {
            pub rule main -> i32 = "calc" v:calc<_>(2, 3) -> { v }
            rule calc(mult: i32, base: i32) -> i32 = i:i32 -> { base + (i * mult) }
        }
    }

    // 10 * 2 + 3 = 23
    multi_args::parse_main
        .parse_str("calc 10")
        .test()
        .assert_success_is(23);
}
