use syn::parse::Parser;
use syn_grammar::testing::Testable;

// Import the modules
pub mod modules;

use modules::rechner::Rechner;

#[test]
fn test_composition_complex() {
    Rechner::parse_main
        .parse_str("rechne ( 2 g + 13kg )")
        .test()
        .assert_success_is(13002000);
}

#[test]
fn test_composition_mixed() {
    Rechner::parse_main
        .parse_str("rechne ( 500mg + 1 g )")
        .test()
        .assert_success_is(1500);
}
