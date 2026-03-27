#![feature(prelude_import)]
#[macro_use]
extern crate std;
#[prelude_import]
use std::prelude::rust_2021::*;
use grammar_kit::WithSpan;
use winnow_grammar::grammar;
use winnow_grammar::testing::WinnowTestExt;
pub struct SpannedInt {
    pub val: u32,
    pub span: std::ops::Range<usize>,
}
#[automatically_derived]
impl ::core::fmt::Debug for SpannedInt {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::debug_struct_field2_finish(
            f,
            "SpannedInt",
            "val",
            &self.val,
            "span",
            &&self.span,
        )
    }
}
#[automatically_derived]
impl ::core::marker::StructuralPartialEq for SpannedInt {}
#[automatically_derived]
impl ::core::cmp::PartialEq for SpannedInt {
    #[inline]
    fn eq(&self, other: &SpannedInt) -> bool {
        self.val == other.val && self.span == other.span
    }
}
#[automatically_derived]
impl ::core::clone::Clone for SpannedInt {
    #[inline]
    fn clone(&self) -> SpannedInt {
        SpannedInt {
            val: ::core::clone::Clone::clone(&self.val),
            span: ::core::clone::Clone::clone(&self.span),
        }
    }
}
impl WithSpan<u32> for SpannedInt {
    fn with_span(val: u32, span: std::ops::Range<usize>) -> Self {
        Self { val, span }
    }
}
#[allow(non_snake_case)]
pub mod SpanTest {
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
            ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
        >,
    > {
        use ::winnow::Parser;
        ::winnow::ascii::multispace0.parse_next(input).map(|_| ())
    }
    #[allow(dead_code)]
    fn parse_main_inner<'a, S: std::fmt::Debug + Clone>(
        input: &mut ::winnow_grammar::ParseInput<'a, S>,
    ) -> ::winnow::Result<
        SpannedInt,
        ::winnow::error::ErrMode<
            ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
        >,
    > {
        use ::winnow::Parser;
        let mut parser = (|
            input: &mut ::winnow_grammar::ParseInput<'a, S>,
        | -> ::winnow::Result<
            SpannedInt,
            ::winnow::error::ErrMode<
                ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
            >,
        > {
            use ::winnow::prelude::*;
            {
                let _ = WS(input)?;
                let start = ::winnow::stream::Location::location(input);
                let n = ::winnow::ascii::dec_uint::<
                    ::winnow_grammar::ParseInput<'a, S>,
                    u32,
                    ::winnow::error::ErrMode<
                        ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
                    >,
                >
                    .parse_next(input)?;
                let end = ::winnow::stream::Location::location(input);
                #[allow(unused_variables)]
                let _span = start..end;
                Ok(<SpannedInt as ::grammar_kit::WithSpan<_>>::with_span({ n }, _span))
            }
        })
            .context(::winnow::error::StrContext::Label("main"));
        { parser.parse_next(input) }
    }
    pub fn parse_main<'a, S: std::fmt::Debug + Clone>() -> impl ::winnow::Parser<
        ::winnow_grammar::ParseInput<'a, S>,
        SpannedInt,
        ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
    > {
        move |
            input: &mut ::winnow_grammar::ParseInput<'a, S>,
        | -> ::winnow::Result<
            SpannedInt,
            ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
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
                        <::winnow::error::InputError<
                            ::winnow_grammar::ParseInput<'a, S>,
                        > as ParserError<
                            ::winnow_grammar::ParseInput<'a, S>,
                        >>::incomplete(input, needed),
                    );
                }
            };
            let result = match parse_main_inner(input) {
                Ok(v) => v,
                Err(
                    ::winnow::error::ErrMode::Backtrack(err)
                    | ::winnow::error::ErrMode::Cut(err),
                ) => return Err(err),
                Err(::winnow::error::ErrMode::Incomplete(needed)) => {
                    return Err(
                        <::winnow::error::InputError<
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
                        <::winnow::error::InputError<
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
                        <::winnow::error::InputError<
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
#[rustc_test_marker = "test_span_injection"]
#[doc(hidden)]
pub const test_span_injection: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_span_injection"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "winnow-grammar/tests/span_injection_test.rs",
        start_line: 25usize,
        start_col: 4usize,
        end_line: 25usize,
        end_col: 23usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_span_injection()),
    ),
};
fn test_span_injection() {
    let input = "  42  ";
    let expected = SpannedInt { val: 42, span: 2..4 };
    SpanTest::parse_main().parse_test(input).assert_success_is(expected);
}
pub struct SpannedTuple {
    pub a: u32,
    pub b: u32,
    pub span: std::ops::Range<usize>,
}
#[automatically_derived]
impl ::core::fmt::Debug for SpannedTuple {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::debug_struct_field3_finish(
            f,
            "SpannedTuple",
            "a",
            &self.a,
            "b",
            &self.b,
            "span",
            &&self.span,
        )
    }
}
#[automatically_derived]
impl ::core::marker::StructuralPartialEq for SpannedTuple {}
#[automatically_derived]
impl ::core::cmp::PartialEq for SpannedTuple {
    #[inline]
    fn eq(&self, other: &SpannedTuple) -> bool {
        self.a == other.a && self.b == other.b && self.span == other.span
    }
}
#[automatically_derived]
impl ::core::clone::Clone for SpannedTuple {
    #[inline]
    fn clone(&self) -> SpannedTuple {
        SpannedTuple {
            a: ::core::clone::Clone::clone(&self.a),
            b: ::core::clone::Clone::clone(&self.b),
            span: ::core::clone::Clone::clone(&self.span),
        }
    }
}
impl WithSpan<(u32, u32)> for SpannedTuple {
    fn with_span(val: (u32, u32), span: std::ops::Range<usize>) -> Self {
        Self { a: val.0, b: val.1, span }
    }
}
#[allow(non_snake_case)]
pub mod SpanTupleTest {
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
            ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
        >,
    > {
        use ::winnow::Parser;
        ::winnow::ascii::multispace0.parse_next(input).map(|_| ())
    }
    #[allow(dead_code)]
    fn parse_main_inner<'a, S: std::fmt::Debug + Clone>(
        input: &mut ::winnow_grammar::ParseInput<'a, S>,
    ) -> ::winnow::Result<
        SpannedTuple,
        ::winnow::error::ErrMode<
            ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
        >,
    > {
        use ::winnow::Parser;
        let mut parser = (|
            input: &mut ::winnow_grammar::ParseInput<'a, S>,
        | -> ::winnow::Result<
            SpannedTuple,
            ::winnow::error::ErrMode<
                ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
            >,
        > {
            use ::winnow::prelude::*;
            {
                let _ = WS(input)?;
                let start = ::winnow::stream::Location::location(input);
                let a = ::winnow::ascii::dec_uint::<
                    ::winnow_grammar::ParseInput<'a, S>,
                    u32,
                    ::winnow::error::ErrMode<
                        ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
                    >,
                >
                    .parse_next(input)?;
                let _ = WS(input)?;
                let b = ::winnow::ascii::dec_uint::<
                    ::winnow_grammar::ParseInput<'a, S>,
                    u32,
                    ::winnow::error::ErrMode<
                        ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
                    >,
                >
                    .parse_next(input)?;
                let end = ::winnow::stream::Location::location(input);
                #[allow(unused_variables)]
                let _span = start..end;
                Ok(
                    <SpannedTuple as ::grammar_kit::WithSpan<
                        _,
                    >>::with_span({ (a, b) }, _span),
                )
            }
        })
            .context(::winnow::error::StrContext::Label("main"));
        { parser.parse_next(input) }
    }
    pub fn parse_main<'a, S: std::fmt::Debug + Clone>() -> impl ::winnow::Parser<
        ::winnow_grammar::ParseInput<'a, S>,
        SpannedTuple,
        ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
    > {
        move |
            input: &mut ::winnow_grammar::ParseInput<'a, S>,
        | -> ::winnow::Result<
            SpannedTuple,
            ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
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
                        <::winnow::error::InputError<
                            ::winnow_grammar::ParseInput<'a, S>,
                        > as ParserError<
                            ::winnow_grammar::ParseInput<'a, S>,
                        >>::incomplete(input, needed),
                    );
                }
            };
            let result = match parse_main_inner(input) {
                Ok(v) => v,
                Err(
                    ::winnow::error::ErrMode::Backtrack(err)
                    | ::winnow::error::ErrMode::Cut(err),
                ) => return Err(err),
                Err(::winnow::error::ErrMode::Incomplete(needed)) => {
                    return Err(
                        <::winnow::error::InputError<
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
                        <::winnow::error::InputError<
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
                        <::winnow::error::InputError<
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
#[rustc_test_marker = "test_span_injection_tuple"]
#[doc(hidden)]
pub const test_span_injection_tuple: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_span_injection_tuple"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "winnow-grammar/tests/span_injection_test.rs",
        start_line: 58usize,
        start_col: 4usize,
        end_line: 58usize,
        end_col: 29usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_span_injection_tuple()),
    ),
};
fn test_span_injection_tuple() {
    let input = " 10 20 ";
    let expected = SpannedTuple {
        a: 10,
        b: 20,
        span: 1..6,
    };
    SpanTupleTest::parse_main().parse_test(input).assert_success_is(expected);
}
#[rustc_main]
#[coverage(off)]
#[doc(hidden)]
pub fn main() -> () {
    extern crate test;
    test::test_main_static(&[&test_span_injection, &test_span_injection_tuple])
}
