use syn::parse::Parser;
use syn_grammar::grammar;
use syn_grammar::testing::Testable;

#[test]
fn test_basic_sequence() {
    grammar! {
        grammar seq_test {
            pub rule main -> (i32, i32) = a:i32 b:i32 -> { (a, b) }
        }
    }

    seq_test::parse_main
        .parse_str("10 20")
        .test()
        .assert_success_is((10, 20));
}
