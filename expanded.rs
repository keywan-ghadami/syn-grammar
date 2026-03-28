    Checking winnow-grammar v0.1.0 (/home/user/syn-grammar/winnow-grammar)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.24s

#![feature(prelude_import)]
#[macro_use]
extern crate std;
#[prelude_import]
use std::prelude::rust_2021::*;
use winnow::Parser;
use winnow_grammar::grammar;
use winnow_grammar::testing::WinnowTestExt;
pub struct CustomNode {
    pub name: String,
    pub span: std::ops::Range<usize>,
}
#[automatically_derived]
impl ::core::fmt::Debug for CustomNode {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::debug_struct_field2_finish(
            f,
            "CustomNode",
            "name",
            &self.name,
            "span",
            &&self.span,
        )
    }
}
#[automatically_derived]
impl ::core::marker::StructuralPartialEq for CustomNode {}
#[automatically_derived]
impl ::core::cmp::PartialEq for CustomNode {
    #[inline]
    fn eq(&self, other: &CustomNode) -> bool {
        self.name == other.name && self.span == other.span
    }
}
impl CustomNode {
    fn from_data(name: String, span: std::ops::Range<usize>) -> Self {
        Self { name, span }
    }
}
#[allow(non_snake_case)]
pub mod ExplicitSpanTest {
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
    fn parse_custom_node_inner<'a, S: std::fmt::Debug + Clone>(
        input: &mut ::winnow_grammar::ParseInput<'a, S>,
    ) -> ::winnow::Result<
        CustomNode,
        ::winnow::error::ErrMode<
            ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
        >,
    > {
        use ::winnow::Parser;
        let mut parser = (|
            input: &mut ::winnow_grammar::ParseInput<'a, S>,
        | -> ::winnow::Result<
            CustomNode,
            ::winnow::error::ErrMode<
                ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
            >,
        > {
            use ::winnow::prelude::*;
            {
                let _ = WS(input)?;
                let start = ::winnow::stream::Location::location(input);
                let a = ::winnow::token::take_while(
                        1..,
                        |c| {
                            ::winnow::stream::AsChar::as_char(c).is_alphanumeric()
                                || ::winnow::stream::AsChar::as_char(c) == '_'
                        },
                    )
                    .parse_next(input)?;
                let end = ::winnow::stream::Location::location(input);
                #[allow(unused_variables)]
                let _span = start..end;
                Ok(CustomNode::from_data(a.to_string(), _span))
            }
        })
            .context(::winnow::error::StrContext::Label("custom_node"));
        { parser.parse_next(input) }
    }
    pub fn parse_custom_node<'a, S: std::fmt::Debug + Clone>() -> impl ::winnow::Parser<
        ::winnow_grammar::ParseInput<'a, S>,
        CustomNode,
        ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
    > {
        move |
            input: &mut ::winnow_grammar::ParseInput<'a, S>,
        | -> ::winnow::Result<
            CustomNode,
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
            let result = match parse_custom_node_inner(input) {
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
#[rustc_test_marker = "test_explicit_span_injection"]
#[doc(hidden)]
pub const test_explicit_span_injection: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_explicit_span_injection"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "winnow-grammar/tests/explicit_span_test.rs",
        start_line: 24usize,
        start_col: 4usize,
        end_line: 24usize,
        end_col: 32usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_explicit_span_injection()),
    ),
};
fn test_explicit_span_injection() {
    let input = "  my_ident  ";
    let result = ExplicitSpanTest::parse_custom_node().parse_test(input).unwrap();
    match (&result.name, &"my_ident") {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
    match (&result.span, &(2..10)) {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
}
#[rustc_main]
#[coverage(off)]
#[doc(hidden)]
pub fn main() -> () {
    extern crate test;
    test::test_main_static(&[&test_explicit_span_injection])
}
