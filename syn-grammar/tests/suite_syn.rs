// Define the backend-specific test_case! macro
macro_rules! test_case {
    ($name:ident, { $($grammar:tt)* }, [ $(($input:expr, $($check:tt)*)),* $(,)? ]) => {
        grammar_kit::test_case_impl!(
            backend: {
                grammar_macro: syn_grammar::grammar,
                test_trait: syn_grammar::SynTestExt,
                parser_mut: // Syn parsers are not mutable
            },
            name: $name,
            grammar: { $($grammar)* },
            cases: [ $( ($input, $($check)*) ),* ]
        );
    };
}

// Include the shared test suite
include!("../../core/grammar-kit/src/common_tests/simple.rs");
