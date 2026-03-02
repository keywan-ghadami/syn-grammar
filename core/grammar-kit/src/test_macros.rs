/// Implementation of the test case logic.
///
/// This macro creates an outer module named after the test for isolation.
///
/// The entry point defaults to `parse_main` but can be customized via the `rule` parameter.
#[macro_export]
macro_rules! test_case_impl {
    // Variant with explicit rule name
    (
        backend: {
            grammar_macro: $grammar_macro:path,
            test_trait: $test_trait:path,
            parser_mut: $($parser_mut:ident)?
        },
        name: $name:ident,
        rule: $rule:ident,
        grammar: { $($grammar:tt)* },
        cases: [ $( ($input:expr, $($check:tt)*) ),* $(,)? ]
    ) => {
        #[allow(non_snake_case)]
        mod $name {
            use paste::paste;
            use $grammar_macro as grammar;
            use $test_trait;

            paste! {
                grammar! { grammar [<$name _grammar>] { $($grammar)* } }
            }

            #[test]
            fn run() {
                paste!{
                    macro_rules! run_check {
                        ($inp:expr, val $expect:expr) => {
                            #[allow(unused_mut)]
                            let $($parser_mut)? parser = [<$name _grammar>]::[<parse_ $rule>];
                            parser.parse_test($inp).assert_success_is($expect);
                        };
                        ($inp:expr, err $msg:expr) => {
                            #[allow(unused_mut)]
                            let $($parser_mut)? parser = [<$name _grammar>]::[<parse_ $rule>];
                            parser.parse_test($inp).assert_failure_contains($msg);
                        };
                        ($inp:expr, check $closure:expr) => {
                            #[allow(unused_mut)]
                            let $($parser_mut)? parser = [<$name _grammar>]::[<parse_ $rule>];
                            parser.parse_test($inp).assert_success_with($closure);
                        };
                    }
                    $(
                        run_check!($input, $($check)*);
                    )*
                }
            }
        }
    };

    // Variant without explicit rule name (defaults to 'main')
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
        $crate::test_case_impl! {
            backend: {
                grammar_macro: $grammar_macro,
                test_trait: $test_trait,
                parser_mut: $($parser_mut)?
            },
            name: $name,
            rule: main, // Default rule name
            grammar: { $($grammar)* },
            cases: [ $(($input, $($check)*)),* ]
        }
    };
}
