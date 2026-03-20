   Compiling winnow-grammar-macros v0.1.0 (/home/user/syn-grammar/winnow-grammar/winnow-grammar-macros)
    Checking winnow-grammar v0.1.0 (/home/user/syn-grammar/winnow-grammar)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.66s

#![feature(prelude_import)]
#[macro_use]
extern crate std;
#[prelude_import]
use std::prelude::rust_2021::*;
use winnow_grammar::grammar;
use winnow_grammar::testing::WinnowTestExt;
#[allow(non_snake_case)]
pub mod Args {
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
        i32,
        ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
    > {
        use ::winnow::Parser;
        let mut parser = (|
            input: &mut ::winnow_grammar::ParseInput<'a, S>,
        | -> ::winnow::Result<
            i32,
            ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
        > {
            use ::winnow::prelude::*;
            {
                let _ = WS(input)?;
                let _ = literal("start")
                    .context(
                        ::winnow::error::StrContext::Expected(
                            ::winnow::error::StrContextValue::StringLiteral("start"),
                        ),
                    )
                    .parse_next(input)?;
                let _ = WS(input)?;
                let v = (|i: &mut ::winnow_grammar::ParseInput<'a, S>| {
                    let mut arg_0 = 10;
                    parse_value_inner(i, &mut arg_0)
                })
                    .parse_next(input)?;
                Ok(v)
            }
        })
            .context(::winnow::error::StrContext::Label("main"));
        { parser.parse_next(input) }
    }
    pub fn parse_main<'a, S: std::fmt::Debug + Clone>() -> impl ::winnow::Parser<
        ::winnow_grammar::ParseInput<'a, S>,
        i32,
        ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
    > {
        move |
            input: &mut ::winnow_grammar::ParseInput<'a, S>,
        | -> ::winnow::Result<
            i32,
            ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
        > {
            let _ = WS(input)?;
            let result = parse_main_inner(input)?;
            let _ = WS(input)?;
            ::winnow::combinator::eof.parse_next(input)?;
            Ok(result)
        }
    }
    #[allow(dead_code)]
    fn parse_value_inner<'a, S: std::fmt::Debug + Clone>(
        input: &mut ::winnow_grammar::ParseInput<'a, S>,
        offset: i32,
    ) -> ::winnow::Result<
        i32,
        ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
    > {
        use ::winnow::Parser;
        let mut parser = (|
            input: &mut ::winnow_grammar::ParseInput<'a, S>,
        | -> ::winnow::Result<
            i32,
            ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
        > {
            use ::winnow::prelude::*;
            {
                let _ = WS(input)?;
                let i = ::winnow::ascii::dec_int::<
                    ::winnow_grammar::ParseInput<'a, S>,
                    i32,
                    ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
                >
                    .parse_next(input)?;
                Ok(i + offset)
            }
        })
            .context(::winnow::error::StrContext::Label("value"));
        { parser.parse_next(input) }
    }
    fn parse_value<'a, S: std::fmt::Debug + Clone>(
        mut offset: i32,
    ) -> impl ::winnow::Parser<
        ::winnow_grammar::ParseInput<'a, S>,
        i32,
        ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
    > {
        move |
            input: &mut ::winnow_grammar::ParseInput<'a, S>,
        | -> ::winnow::Result<
            i32,
            ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
        > {
            let _ = WS(input)?;
            let result = parse_value_inner(input, offset.clone())?;
            let _ = WS(input)?;
            ::winnow::combinator::eof.parse_next(input)?;
            Ok(result)
        }
    }
}
extern crate test;
#[rustc_test_marker = "test_args"]
#[doc(hidden)]
pub const test_args: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_args"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "winnow-grammar/tests/args.rs",
        start_line: 15usize,
        start_col: 4usize,
        end_line: 15usize,
        end_col: 13usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(#[coverage(off)] || test::assert_test_result(test_args())),
};
fn test_args() {
    Args::parse_main().parse_test("start 5").assert_success_is(15);
}
#[rustc_main]
#[coverage(off)]
#[doc(hidden)]
pub fn main() -> () {
    extern crate test;
    test::test_main_static(&[&test_args])
}
