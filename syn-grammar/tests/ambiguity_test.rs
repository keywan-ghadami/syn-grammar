use syn::parse::Parser;
use syn_grammar::grammar;
use syn_grammar::testing::Testable;

#[test]
fn test_group_vs_args_ambiguity() {
    grammar! {
        grammar test_ambiguity {
            pub main -> String = item ("a") -> { "success".to_string() }
            item = "item"
        }
    }

    test_ambiguity::parse_main
        .parse_str("item a")
        .test()
        .assert_success_is("success".to_string());
}
