use syn::parse::Parser;
use syn_grammar::grammar;
use syn_grammar::testing::Testable;

grammar! {
    grammar LitBindWithAction {
        pub rule main -> (String, String) = a:"a" b:"b" -> { (a.to_string(), b.to_string()) }
    }
}

#[test]
fn test_literal_binding_with_action() {
    LitBindWithAction::parse_main
        .parse_str("a b")
        .test()
        .assert_success_is(("a".to_string(), "b".to_string()));
}
