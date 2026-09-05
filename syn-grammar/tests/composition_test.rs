use syn::parse::Parser;
use syn_grammar::testing::Testable;

// Import the modules
pub mod modules;

use modules::calculator::Calculator;

#[test]
fn test_composition_complex() {
    Calculator::parse_main
        .parse_str("calculate ( 2 g + 13 kg )")
        .test()
        .assert_success_is(13002000);
}

#[test]
fn test_composition_mixed() {
    Calculator::parse_main
        .parse_str("calculate ( 500 mg + 1 g )")
        .test()
        .assert_success_is(1500);
}
