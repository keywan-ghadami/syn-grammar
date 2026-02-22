use syn::parse::Parser;
use syn_grammar::grammar;
use syn_grammar::testing::Testable;

grammar! {
    grammar plus_val {
        pub rule list -> Vec<i32> = i:i32+ -> { i }
    }
}

#[test]
fn test_plus_operator_validation() {
    plus_val::parse_list
        .parse_str("1 2")
        .test()
        .assert_success_is(vec![1, 2]);

    plus_val::parse_list
        .parse_str("")
        .test()
        .assert_failure_contains("expected integer");
}
