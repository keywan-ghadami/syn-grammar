use syn::parse::Parser;
use syn_grammar::grammar;
use syn_grammar::testing::Testable;

grammar! {
    grammar repetition_test {
        pub rule star -> Vec<i32> = items:i32* -> { items }
        pub rule plus -> Vec<i32> = items:i32+ -> { items }
        pub rule optional -> Option<i32> = item:i32? -> { item }
    }
}

#[test]
fn test_repetition() {
    repetition_test::parse_star
        .parse_str("")
        .test()
        .assert_success_is(vec![]);

    repetition_test::parse_star
        .parse_str("1 2")
        .test()
        .assert_success_is(vec![1, 2]);

    repetition_test::parse_plus
        .parse_str("1")
        .test()
        .assert_success_is(vec![1]);

    repetition_test::parse_plus
        .parse_str("1 2")
        .test()
        .assert_success_is(vec![1, 2]);

    repetition_test::parse_plus
        .parse_str("")
        .test()
        .assert_failure();

    repetition_test::parse_optional
        .parse_str("")
        .test()
        .assert_success_is(None);

    repetition_test::parse_optional
        .parse_str("1")
        .test()
        .assert_success_is(Some(1));
}
