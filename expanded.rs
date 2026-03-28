    Checking winnow-grammar v0.1.0 (/home/user/syn-grammar/winnow-grammar)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.94s

#![feature(prelude_import)]
#[macro_use]
extern crate std;
#[prelude_import]
use std::prelude::rust_2021::*;
use winnow_grammar::grammar;
use winnow_grammar::testing::WinnowTestExt;
#[allow(non_snake_case)]
pub mod OctParser {
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
        ::winnow::error::ErrMode<
            ::winnow::error::ContextError<::winnow_grammar::ParseInput<'a, S>>,
        >,
    > {
        use ::winnow::Parser;
        ::winnow::ascii::multispace0.parse_next(input).map(|_| ())
    }
    #[allow(dead_code)]
    fn parse_test_oct_inner<'a, S: std::fmt::Debug + Clone>(
        input: &mut ::winnow_grammar::ParseInput<'a, S>,
    ) -> ::winnow::Result<
        String,
        ::winnow::error::ErrMode<
            ::winnow::error::ContextError<::winnow_grammar::ParseInput<'a, S>>,
        >,
    > {
        use ::winnow::Parser;
        let mut parser = (|
            input: &mut ::winnow_grammar::ParseInput<'a, S>,
        | -> ::winnow::Result<
            String,
            ::winnow::error::ErrMode<
                ::winnow::error::ContextError<::winnow_grammar::ParseInput<'a, S>>,
            >,
        > {
            use ::winnow::prelude::*;
            {
                let _ = WS(input)?;
                let o = ::winnow::ascii::oct_digit1::<
                    ::winnow_grammar::ParseInput<'a, S>,
                    ::winnow::error::ErrMode<
                        ::winnow::error::ContextError<
                            ::winnow_grammar::ParseInput<'a, S>,
                        >,
                    >,
                >
                    .parse_next(input)?;
                Ok(o.to_string())
            }
        })
            .context(::winnow::error::StrContext::Label("test_oct"));
        { parser.parse_next(input) }
    }
    pub fn parse_test_oct<'a, S: std::fmt::Debug + Clone>() -> impl ::winnow::Parser<
        ::winnow_grammar::ParseInput<'a, S>,
        String,
        ::winnow::error::ContextError<::winnow_grammar::ParseInput<'a, S>>,
    > {
        move |
            input: &mut ::winnow_grammar::ParseInput<'a, S>,
        | -> ::winnow::Result<
            String,
            ::winnow::error::ContextError<::winnow_grammar::ParseInput<'a, S>>,
        > {
            use ::winnow::error::ParserError;
            match WS(input) {
                Ok(v) => v,
                Err(
                    ::winnow::error::ErrMode::Backtrack(err)
                    | ::winnow::error::ErrMode::Cut(err),
                ) => return Err(err),
                Err(::winnow::error::ErrMode::Incomplete(needed)) => {
                    return Err(
                        <::winnow::error::ContextError<
                            ::winnow_grammar::ParseInput<'a, S>,
                        > as ParserError<
                            ::winnow_grammar::ParseInput<'a, S>,
                        >>::incomplete(input, needed),
                    );
                }
            };
            let result = match parse_test_oct_inner(input) {
                Ok(v) => v,
                Err(
                    ::winnow::error::ErrMode::Backtrack(err)
                    | ::winnow::error::ErrMode::Cut(err),
                ) => return Err(err),
                Err(::winnow::error::ErrMode::Incomplete(needed)) => {
                    return Err(
                        <::winnow::error::ContextError<
                            ::winnow_grammar::ParseInput<'a, S>,
                        > as ParserError<
                            ::winnow_grammar::ParseInput<'a, S>,
                        >>::incomplete(input, needed),
                    );
                }
            };
            match WS(input) {
                Ok(v) => v,
                Err(
                    ::winnow::error::ErrMode::Backtrack(err)
                    | ::winnow::error::ErrMode::Cut(err),
                ) => return Err(err),
                Err(::winnow::error::ErrMode::Incomplete(needed)) => {
                    return Err(
                        <::winnow::error::ContextError<
                            ::winnow_grammar::ParseInput<'a, S>,
                        > as ParserError<
                            ::winnow_grammar::ParseInput<'a, S>,
                        >>::incomplete(input, needed),
                    );
                }
            };
            match ::winnow::combinator::eof.parse_next(input) {
                Ok(v) => v,
                Err(
                    ::winnow::error::ErrMode::Backtrack(err)
                    | ::winnow::error::ErrMode::Cut(err),
                ) => return Err(err),
                Err(::winnow::error::ErrMode::Incomplete(needed)) => {
                    return Err(
                        <::winnow::error::ContextError<
                            ::winnow_grammar::ParseInput<'a, S>,
                        > as ParserError<
                            ::winnow_grammar::ParseInput<'a, S>,
                        >>::incomplete(input, needed),
                    );
                }
            };
            Ok(result)
        }
    }
}
extern crate test;
#[rustc_test_marker = "test_oct_literal"]
#[doc(hidden)]
pub const test_oct_literal: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_oct_literal"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "winnow-grammar/tests/oct_test.rs",
        start_line: 12usize,
        start_col: 4usize,
        end_line: 12usize,
        end_col: 20usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_oct_literal()),
    ),
};
fn test_oct_literal() {
    OctParser::parse_test_oct()
        .parse_test("1234567")
        .assert_success_is("1234567".to_string());
    OctParser::parse_test_oct().parse_test("0").assert_success_is("0".to_string());
    OctParser::parse_test_oct().parse_test("8").assert_failure();
}
#[rustc_main]
#[coverage(off)]
#[doc(hidden)]
pub fn main() -> () {
    extern crate test;
    test::test_main_static(&[&test_oct_literal])
}
