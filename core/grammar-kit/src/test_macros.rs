/// Implementation of the test case logic for the Include Pattern.
///
/// This macro defines the grammar *inside* the test function to ensure isolation
/// without needing external wrapper modules. It preserves the grammar name given by `$name`.
#[macro_export]
macro_rules! test_case_impl {
    (
        backend: {
            grammar_macro: $grammar_macro:path,
            test_trait: $test_trait:path,
            parser_mut: $($parser_mut:ident)?
        },
        name: $name:ident,
        grammar: { $($grammar:tt)* },
        cases: [ $( ($input:expr, $($check:tt)*) ),* $(,)? ]
    ) => {
        #[test]
        fn $name() {
            // 1. Import the backend's grammar macro locally
            use $grammar_macro as grammar;

            // 2. Define the grammar inside the function scope.
            //    This creates a module named `$name` (e.g., `mod simple_return`).
            grammar! { grammar $name { $($grammar)* } }

            // 3. Import the specific TestExtension trait (SynTestExt or WinnowTestExt)
            use $test_trait;

            // 4. Local helper to dispatch the check logic (Value vs Error vs Closure)
            macro_rules! run_check {
                ($inp:expr, val $expect:expr) => {
                    #[allow(unused_mut)]
                    // We access the parser via the local module `$name`
                    let $($parser_mut)? parser = $name::parse_main;
                    parser.parse_test($inp).assert_success_is($expect);
                };
                ($inp:expr, err $msg:expr) => {
                    #[allow(unused_mut)]
                    let $($parser_mut)? parser = $name::parse_main;
                    parser.parse_test($inp).assert_failure_contains($msg);
                };
                ($inp:expr, check $closure:expr) => {
                    #[allow(unused_mut)]
                    let $($parser_mut)? parser = $name::parse_main;
                    parser.parse_test($inp).assert_success_with($closure);
                };
            }

            // 5. Run the checks provided in the cases array
            $(
                run_check!($input, $($check)*);
            )*
        }
    };
}
