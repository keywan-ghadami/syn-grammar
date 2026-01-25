use syn_grammar::include_grammar;
use syn_grammar::testing::Testable;
use syn::parse::Parser;

// Include the grammar from the fixtures directory
include_grammar!("tests/fixtures/simple.g");

#[test]
fn test_included_grammar() {
    // The macro generates the module 'SimpleIncluded'
    SimpleIncluded::parse_main.parse_str("hello")
        .test()
        .assert_success_is("world");
}
