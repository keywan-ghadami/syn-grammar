use syn::parse::Parser;
use syn_grammar::grammar;
use syn_grammar::testing::Testable;

grammar! {
    grammar LitBinNoAction {
        // This rule has no action block and no trailing semicolon.
        // It should implicitly return a tuple of its bound literal tokens.
        // We must specify the return type since it defaults to ().
        pub rule main -> (kw::a, kw::b) = a:"a" b:"b"
    }
}

#[test]
fn test_literal_binding_no_action() {
    LitBinNoAction::parse_main
        .parse_str("a b")
        .test()
        .assert_success_is((
            LitBinNoAction::kw::a::default(),
            LitBinNoAction::kw::b::default(),
        ));
}
