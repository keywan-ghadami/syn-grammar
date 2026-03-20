    Checking winnow-grammar v0.1.0 (/home/user/syn-grammar/winnow-grammar)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 5.88s

#![feature(prelude_import)]
#[macro_use]
extern crate std;
#[prelude_import]
use std::prelude::rust_2021::*;
use winnow_grammar::grammar;
use winnow_grammar::testing::WinnowTestExt;
pub enum Expr {
    Num(u32),
    Add(Box<Expr>, Box<Expr>),
}
#[automatically_derived]
impl ::core::fmt::Debug for Expr {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match self {
            Expr::Num(__self_0) => {
                ::core::fmt::Formatter::debug_tuple_field1_finish(f, "Num", &__self_0)
            }
            Expr::Add(__self_0, __self_1) => {
                ::core::fmt::Formatter::debug_tuple_field2_finish(
                    f,
                    "Add",
                    __self_0,
                    &__self_1,
                )
            }
        }
    }
}
#[automatically_derived]
impl ::core::marker::StructuralPartialEq for Expr {}
#[automatically_derived]
impl ::core::cmp::PartialEq for Expr {
    #[inline]
    fn eq(&self, other: &Expr) -> bool {
        let __self_discr = ::core::intrinsics::discriminant_value(self);
        let __arg1_discr = ::core::intrinsics::discriminant_value(other);
        __self_discr == __arg1_discr
            && match (self, other) {
                (Expr::Num(__self_0), Expr::Num(__arg1_0)) => __self_0 == __arg1_0,
                (Expr::Add(__self_0, __self_1), Expr::Add(__arg1_0, __arg1_1)) => {
                    __self_0 == __arg1_0 && __self_1 == __arg1_1
                }
                _ => unsafe { ::core::intrinsics::unreachable() }
            }
    }
}
#[automatically_derived]
impl ::core::clone::Clone for Expr {
    #[inline]
    fn clone(&self) -> Expr {
        match self {
            Expr::Num(__self_0) => Expr::Num(::core::clone::Clone::clone(__self_0)),
            Expr::Add(__self_0, __self_1) => {
                Expr::Add(
                    ::core::clone::Clone::clone(__self_0),
                    ::core::clone::Clone::clone(__self_1),
                )
            }
        }
    }
}
#[allow(non_snake_case)]
pub mod LeftRec {
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
    fn parse_expr_inner<'a, S: std::fmt::Debug + Clone>(
        input: &mut ::winnow_grammar::ParseInput<'a, S>,
    ) -> ::winnow::Result<
        Expr,
        ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
    > {
        use ::winnow::Parser;
        let mut parser = (|
            input: &mut ::winnow_grammar::ParseInput<'a, S>,
        | -> ::winnow::Result<
            Expr,
            ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
        > {
            use ::winnow::prelude::*;
            let mut lhs = {
                let _ = WS(input)?;
                let t = (|i: &mut ::winnow_grammar::ParseInput<'a, S>| parse_term_inner(
                    i,
                ))
                    .parse_next(input)?;
                Ok(t)
            }?;
            loop {
                {
                    let checkpoint = ::winnow::stream::Stream::checkpoint(input);
                    let attempt = (|| {
                        let _ = WS(input)?;
                        let _ = literal("+")
                            .context(
                                ::winnow::error::StrContext::Expected(
                                    ::winnow::error::StrContextValue::StringLiteral("+"),
                                ),
                            )
                            .parse_next(input)?;
                        let _ = WS(input)?;
                        let r = (|i: &mut ::winnow_grammar::ParseInput<'a, S>| parse_term_inner(
                            i,
                        ))
                            .parse_next(input)?;
                        let l = lhs.clone();
                        Ok(Expr::Add(Box::new(l), Box::new(r)))
                    })();
                    match attempt {
                        Ok(val) => {
                            lhs = val;
                            continue;
                        }
                        Err(e) => {
                            if #[allow(non_exhaustive_omitted_patterns)]
                            match e {
                                ::winnow::error::ErrMode::Cut(_) => true,
                                _ => false,
                            } {
                                return Err(e);
                            } else {
                                ::winnow::stream::Stream::reset(input, &checkpoint);
                            }
                        }
                    }
                }
                break;
            }
            Ok(lhs)
        })
            .context(::winnow::error::StrContext::Label("expr"));
        { parser.parse_next(input) }
    }
    pub fn parse_expr<'a, S: std::fmt::Debug + Clone>() -> impl ::winnow::Parser<
        ::winnow_grammar::ParseInput<'a, S>,
        Expr,
        ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
    > {
        move |
            input: &mut ::winnow_grammar::ParseInput<'a, S>,
        | -> ::winnow::Result<
            Expr,
            ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
        > {
            let _ = WS(input)?;
            let result = parse_expr_inner(input)?;
            let _ = WS(input)?;
            ::winnow::combinator::eof.parse_next(input)?;
            Ok(result)
        }
    }
    #[allow(dead_code)]
    fn parse_term_inner<'a, S: std::fmt::Debug + Clone>(
        input: &mut ::winnow_grammar::ParseInput<'a, S>,
    ) -> ::winnow::Result<
        Expr,
        ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
    > {
        use ::winnow::Parser;
        let mut parser = (|
            input: &mut ::winnow_grammar::ParseInput<'a, S>,
        | -> ::winnow::Result<
            Expr,
            ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
        > {
            use ::winnow::prelude::*;
            {
                let _ = WS(input)?;
                let n = ::winnow::ascii::dec_uint::<
                    ::winnow_grammar::ParseInput<'a, S>,
                    u32,
                    ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
                >
                    .parse_next(input)?;
                Ok(Expr::Num(n))
            }
        })
            .context(::winnow::error::StrContext::Label("term"));
        { parser.parse_next(input) }
    }
    fn parse_term<'a, S: std::fmt::Debug + Clone>() -> impl ::winnow::Parser<
        ::winnow_grammar::ParseInput<'a, S>,
        Expr,
        ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
    > {
        move |
            input: &mut ::winnow_grammar::ParseInput<'a, S>,
        | -> ::winnow::Result<
            Expr,
            ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
        > {
            let _ = WS(input)?;
            let result = parse_term_inner(input)?;
            let _ = WS(input)?;
            ::winnow::combinator::eof.parse_next(input)?;
            Ok(result)
        }
    }
}
extern crate test;
#[rustc_test_marker = "test_left_recursion"]
#[doc(hidden)]
pub const test_left_recursion: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_left_recursion"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "winnow-grammar/tests/left_recursion.rs",
        start_line: 22usize,
        start_col: 4usize,
        end_line: 22usize,
        end_col: 23usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_left_recursion()),
    ),
};
fn test_left_recursion() {
    LeftRec::parse_expr()
        .parse_test("1 + 2 + 3")
        .assert_success_is(
            Expr::Add(
                Box::new(Expr::Add(Box::new(Expr::Num(1)), Box::new(Expr::Num(2)))),
                Box::new(Expr::Num(3)),
            ),
        );
}
#[rustc_main]
#[coverage(off)]
#[doc(hidden)]
pub fn main() -> () {
    extern crate test;
    test::test_main_static(&[&test_left_recursion])
}
