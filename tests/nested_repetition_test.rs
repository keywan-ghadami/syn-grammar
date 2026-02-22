use syn::parse::Parser;
use syn_grammar::grammar;
use syn_grammar::testing::Testable;

grammar! {
    grammar nested_repetition_test {
        pub rule main -> Vec<Vec<i32>> =
            groups:group* -> { groups }

        rule group -> Vec<i32> =
            paren(items:i32*) -> { items }
    }
}

#[test]
fn test_nested_repetition() {
    nested_repetition_test::parse_main
        .parse_str("(1 2) (3)")
        .test()
        .assert_success_is(vec![vec![1, 2], vec![3]]);
}
