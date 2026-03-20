    Checking winnow-grammar v0.1.0 (/home/user/syn-grammar/winnow-grammar)
error[E0425]: cannot find type `T` in this scope
 --> winnow-grammar/tests/generics_test.rs:7:39
  |
7 | ...-> Vec<T> =
  |           ^ not found in this scope
error[E0425]: cannot find value `elements` in this scope
 --> winnow-grammar/tests/generics_test.rs:8:33
  |
8 | ...-> { elements }
  |         ^^^^^^^^ not found in this scope
For more information about this error, try `rustc --explain E0425`.

#![feature(prelude_import)]
#[macro_use]
extern crate std;
#[prelude_import]
use std::prelude::rust_2021::*;
use winnow::prelude::*;
use winnow_grammar::grammar;
use winnow_grammar::testing::WinnowTestExt;
#[allow(non_snake_case)]
pub mod Generics {
    #![allow(unused_imports)]
    #![allow(dead_code)]
    use super::*;
    use ::winnow::prelude::*;
    use ::winnow::token::literal;
    use ::winnow::combinator::{alt, repeat, opt, delimited, preceded};
    #[allow(dead_code)]
    fn WS<'a, S: std::fmt::Debug + Clone>(
        input: &mut ::winnow_grammar::ParseInput<'a, S>,
    ) -> ::winnow::Result<
        (),
        ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
    > {
        use ::winnow::Parser;
        ::winnow::ascii::multispace0.parse_next(input).map(|_| ())
    }
    #[allow(dead_code)]
    fn parse_main_inner<'a, S: std::fmt::Debug + Clone>(
        input: &mut ::winnow_grammar::ParseInput<'a, S>,
    ) -> ::winnow::Result<
        Vec<u32>,
        ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
    > {
        use ::winnow::Parser;
        let mut parser = (|
            input: &mut ::winnow_grammar::ParseInput<'a, S>,
        | -> ::winnow::Result<
            Vec<u32>,
            ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
        > {
            use ::winnow::prelude::*;
            {
                let _ = WS(input)?;
                let l = (|
                    i: &mut ::winnow_grammar::ParseInput<'a, S>,
                | -> ::winnow::Result<
                    Vec<T>,
                    ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
                > {
                    let mut parser = (|
                        i: &mut ::winnow_grammar::ParseInput<'a, S>,
                    | -> ::winnow::Result<
                        Vec<T>,
                        ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
                    > {
                        {
                            let _ = WS(input)?;
                            let _: () = ::winnow::combinator::repeat::<
                                _,
                                _,
                                (),
                                _,
                                _,
                            >(
                                    0..,
                                    ::winnow::combinator::preceded(
                                        |i: &mut ::winnow_grammar::ParseInput<'a, S>| WS(i),
                                        ::winnow::ascii::dec_uint::<
                                            ::winnow_grammar::ParseInput<'a, S>,
                                            u32,
                                            ::winnow::error::InputError<
                                                ::winnow_grammar::ParseInput<'a, S>,
                                            >,
                                        >,
                                    ),
                                )
                                .parse_next(input)?;
                            Ok(elements)
                        }
                    });
                    ::winnow::Parser::parse_next(&mut parser, i)
                })
                    .parse_next(input)?;
                Ok(l)
            }
        })
            .context(::winnow::error::StrContext::Label("main"));
        { parser.parse_next(input) }
    }
    pub fn parse_main<'a, S: std::fmt::Debug + Clone>() -> impl ::winnow::Parser<
        ::winnow_grammar::ParseInput<'a, S>,
        Vec<u32>,
        ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
    > {
        move |
            input: &mut ::winnow_grammar::ParseInput<'a, S>,
        | -> ::winnow::Result<
            Vec<u32>,
            ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
        > {
            let _ = WS(input)?;
            let result = parse_main_inner(input)?;
            let _ = WS(input)?;
            ::winnow::combinator::eof.parse_next(input)?;
            Ok(result)
        }
    }
}
extern crate test;
#[rustc_test_marker = "test_generics"]
#[doc(hidden)]
pub const test_generics: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_generics"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "winnow-grammar/tests/generics_test.rs",
        start_line: 14usize,
        start_col: 4usize,
        end_line: 14usize,
        end_col: 17usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_generics()),
    ),
};
fn test_generics() {
    Generics::parse_main()
        .parse_test("1 2 3")
        .assert_success_is(<[_]>::into_vec(::alloc::boxed::box_new([1, 2, 3])));
}
#[rustc_main]
#[coverage(off)]
#[doc(hidden)]
pub fn main() -> () {
    extern crate test;
    test::test_main_static(&[&test_generics])
}
