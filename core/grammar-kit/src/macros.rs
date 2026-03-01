/// Macro to define a test that runs against both syn-grammar and winnow-grammar.
///
/// This macro generates two test modules, one for `syn` and one for `winnow`.
/// It takes the grammar definition and the test logic as input.
///
/// # Usage
///
/// ```rust
/// use grammar_kit::test_both_backends;
///
/// test_both_backends! {
///     grammar MyGrammar {
///         pub rule main -> i32 = "a" -> { 1 }
///     }
///     test(parse_main) {
///         parse_main.parse_test("a").assert_success_is(1);
///     }
/// }
/// ```
#[macro_export]
macro_rules! test_both_backends {
    (
        grammar $grammar_name:ident {
            $($grammar_body:tt)*
        }
        test($parser_name:ident) {
            $($test_body:tt)*
        }
    ) => {
        // Module for syn-grammar tests
        #[cfg(feature = "syn-backend")]
        mod syn_tests {
            use super::*;
            use syn_grammar::grammar;
            use syn_grammar::SynTestExt;

            grammar! {
                grammar $grammar_name {
                    $($grammar_body)*
                }
            }

            #[test]
            fn test_syn() {
                // Alias the parser function to the name expected by the test body
                let $parser_name = $grammar_name::$parser_name;
                $($test_body)*
            }
        }

        // Module for winnow-grammar tests
        #[cfg(feature = "winnow-backend")]
        mod winnow_tests {
            use super::*;
            use winnow_grammar::grammar;
            use winnow_grammar::testing::WinnowTestExt;

            grammar! {
                grammar $grammar_name {
                    $($grammar_body)*
                }
            }

            #[test]
            fn test_winnow() {
                 // Alias the parser function to the name expected by the test body
                let mut $parser_name = $grammar_name::$parser_name;
                $($test_body)*
            }
        }
    };
}
