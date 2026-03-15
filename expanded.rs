    Checking winnow-grammar v0.1.0 (/home/user/syn-grammar/winnow-grammar)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.73s

#![feature(prelude_import)]
#[macro_use]
extern crate std;
#[prelude_import]
use std::prelude::rust_2021::*;
use winnow_grammar::grammar;
use winnow_grammar::testing::WinnowTestExt;
pub enum Value {
    Int(i32),
    Float(f64),
    String(String),
    Bool(bool),
    List(Vec<Value>),
}
#[automatically_derived]
impl ::core::fmt::Debug for Value {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match self {
            Value::Int(__self_0) => {
                ::core::fmt::Formatter::debug_tuple_field1_finish(f, "Int", &__self_0)
            }
            Value::Float(__self_0) => {
                ::core::fmt::Formatter::debug_tuple_field1_finish(f, "Float", &__self_0)
            }
            Value::String(__self_0) => {
                ::core::fmt::Formatter::debug_tuple_field1_finish(f, "String", &__self_0)
            }
            Value::Bool(__self_0) => {
                ::core::fmt::Formatter::debug_tuple_field1_finish(f, "Bool", &__self_0)
            }
            Value::List(__self_0) => {
                ::core::fmt::Formatter::debug_tuple_field1_finish(f, "List", &__self_0)
            }
        }
    }
}
#[automatically_derived]
impl ::core::marker::StructuralPartialEq for Value {}
#[automatically_derived]
impl ::core::cmp::PartialEq for Value {
    #[inline]
    fn eq(&self, other: &Value) -> bool {
        let __self_discr = ::core::intrinsics::discriminant_value(self);
        let __arg1_discr = ::core::intrinsics::discriminant_value(other);
        __self_discr == __arg1_discr
            && match (self, other) {
                (Value::Int(__self_0), Value::Int(__arg1_0)) => __self_0 == __arg1_0,
                (Value::Float(__self_0), Value::Float(__arg1_0)) => __self_0 == __arg1_0,
                (Value::String(__self_0), Value::String(__arg1_0)) => {
                    __self_0 == __arg1_0
                }
                (Value::Bool(__self_0), Value::Bool(__arg1_0)) => __self_0 == __arg1_0,
                (Value::List(__self_0), Value::List(__arg1_0)) => __self_0 == __arg1_0,
                _ => unsafe { ::core::intrinsics::unreachable() }
            }
    }
}
#[allow(non_snake_case)]
pub mod Comprehensive {
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
    fn parse_value_inner<'a, S: std::fmt::Debug + Clone>(
        input: &mut ::winnow_grammar::ParseInput<'a, S>,
    ) -> ::winnow::Result<
        Value,
        ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
    > {
        use ::winnow::Parser;
        let mut parser = (|
            input: &mut ::winnow_grammar::ParseInput<'a, S>,
        | -> ::winnow::Result<
            Value,
            ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
        > {
            use ::winnow::prelude::*;
            alt((
                    |
                        input: &mut ::winnow_grammar::ParseInput<'a, S>,
                    | -> ::winnow::Result<_> {
                        let _ = WS(input)?;
                        let i = ::winnow::ascii::dec_int::<
                            ::winnow_grammar::ParseInput<'a, S>,
                            i32,
                            ::winnow::error::InputError<
                                ::winnow_grammar::ParseInput<'a, S>,
                            >,
                        >
                            .parse_next(input)?;
                        let _ = WS(input)?;
                        let _ = ::winnow::combinator::not(
                                literal(".")
                                    .context(
                                        ::winnow::error::StrContext::Expected(
                                            ::winnow::error::StrContextValue::StringLiteral("."),
                                        ),
                                    ),
                            )
                            .parse_next(input)?;
                        let _ = WS(input)?;
                        let _ = ::winnow::combinator::not(
                                literal("e")
                                    .context(
                                        ::winnow::error::StrContext::Expected(
                                            ::winnow::error::StrContextValue::StringLiteral("e"),
                                        ),
                                    ),
                            )
                            .parse_next(input)?;
                        let _ = WS(input)?;
                        let _ = ::winnow::combinator::not(
                                literal("E")
                                    .context(
                                        ::winnow::error::StrContext::Expected(
                                            ::winnow::error::StrContextValue::StringLiteral("E"),
                                        ),
                                    ),
                            )
                            .parse_next(input)?;
                        Ok({ Value::Int(i) })
                    },
                    |
                        input: &mut ::winnow_grammar::ParseInput<'a, S>,
                    | -> ::winnow::Result<_> {
                        let _ = WS(input)?;
                        let f = ::winnow::ascii::float::<
                            ::winnow_grammar::ParseInput<'a, S>,
                            f64,
                            ::winnow::error::InputError<
                                ::winnow_grammar::ParseInput<'a, S>,
                            >,
                        >
                            .parse_next(input)?;
                        Ok({ Value::Float(f) })
                    },
                    |
                        input: &mut ::winnow_grammar::ParseInput<'a, S>,
                    | -> ::winnow::Result<_> {
                        let _ = WS(input)?;
                        let s = delimited(
                                '"',
                                ::winnow::ascii::take_escaped(
                                    ::winnow::token::none_of(['\\', '"']),
                                    '\\',
                                    ::winnow::token::one_of(['\\', '"']),
                                ),
                                '"',
                            )
                            .parse_next(input)?;
                        Ok({ Value::String(s.to_string()) })
                    },
                    |
                        input: &mut ::winnow_grammar::ParseInput<'a, S>,
                    | -> ::winnow::Result<_> {
                        let _ = WS(input)?;
                        let _ = literal("true")
                            .context(
                                ::winnow::error::StrContext::Expected(
                                    ::winnow::error::StrContextValue::StringLiteral("true"),
                                ),
                            )
                            .parse_next(input)?;
                        Ok({ Value::Bool(true) })
                    },
                    |
                        input: &mut ::winnow_grammar::ParseInput<'a, S>,
                    | -> ::winnow::Result<_> {
                        let _ = WS(input)?;
                        let _ = literal("false")
                            .context(
                                ::winnow::error::StrContext::Expected(
                                    ::winnow::error::StrContextValue::StringLiteral("false"),
                                ),
                            )
                            .parse_next(input)?;
                        Ok({ Value::Bool(false) })
                    },
                    |
                        input: &mut ::winnow_grammar::ParseInput<'a, S>,
                    | -> ::winnow::Result<_> {
                        let _ = WS(input)?;
                        let _ = literal("[")
                            .context(
                                ::winnow::error::StrContext::Expected(
                                    ::winnow::error::StrContextValue::StringLiteral("["),
                                ),
                            )
                            .parse_next(input)?;
                        let _ = WS(input)?;
                        let l = (move |i: &mut _| parse_list_content_inner(i))
                            .parse_next(input)?;
                        let _ = WS(input)?;
                        let _ = literal("]")
                            .context(
                                ::winnow::error::StrContext::Expected(
                                    ::winnow::error::StrContextValue::StringLiteral("]"),
                                ),
                            )
                            .parse_next(input)?;
                        Ok({ Value::List(l) })
                    },
                ))
                .parse_next(input)
        })
            .context(::winnow::error::StrContext::Label("value"));
        { parser.parse_next(input) }
    }
    pub fn parse_value<'a, S: std::fmt::Debug + Clone>() -> impl ::winnow::Parser<
        ::winnow_grammar::ParseInput<'a, S>,
        Value,
        ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
    > {
        move |
            input: &mut ::winnow_grammar::ParseInput<'a, S>,
        | -> ::winnow::Result<
            Value,
            ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
        > {
            let _ = WS(input)?;
            let result = parse_value_inner(input)?;
            let _ = WS(input)?;
            ::winnow::combinator::eof.parse_next(input)?;
            Ok(result)
        }
    }
    #[allow(dead_code)]
    fn parse_list_content_inner<'a, S: std::fmt::Debug + Clone>(
        input: &mut ::winnow_grammar::ParseInput<'a, S>,
    ) -> ::winnow::Result<
        Vec<Value>,
        ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
    > {
        use ::winnow::Parser;
        let mut parser = (|
            input: &mut ::winnow_grammar::ParseInput<'a, S>,
        | -> ::winnow::Result<
            Vec<Value>,
            ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
        > {
            use ::winnow::prelude::*;
            alt((
                    |
                        input: &mut ::winnow_grammar::ParseInput<'a, S>,
                    | -> ::winnow::Result<_> {
                        let _ = WS(input)?;
                        let v = (move |i: &mut _| parse_value_inner(i))
                            .parse_next(input)?;
                        let _ = WS(input)?;
                        let _ = literal(",")
                            .context(
                                ::winnow::error::StrContext::Expected(
                                    ::winnow::error::StrContextValue::StringLiteral(","),
                                ),
                            )
                            .parse_next(input)?;
                        let _ = WS(input)?;
                        let l = (move |i: &mut _| parse_list_content_inner(i))
                            .parse_next(input)?;
                        Ok({
                            let mut l = l;
                            l.insert(0, v);
                            l
                        })
                    },
                    |
                        input: &mut ::winnow_grammar::ParseInput<'a, S>,
                    | -> ::winnow::Result<_> {
                        let _ = WS(input)?;
                        let v = (move |i: &mut _| parse_value_inner(i))
                            .parse_next(input)?;
                        Ok({ <[_]>::into_vec(::alloc::boxed::box_new([v])) })
                    },
                    |
                        input: &mut ::winnow_grammar::ParseInput<'a, S>,
                    | -> ::winnow::Result<_> {
                        let _ = WS(input)?;
                        let _ = ::winnow::combinator::empty.parse_next(input)?;
                        Ok({ ::alloc::vec::Vec::new() })
                    },
                ))
                .parse_next(input)
        })
            .context(::winnow::error::StrContext::Label("list_content"));
        { parser.parse_next(input) }
    }
    fn parse_list_content<'a, S: std::fmt::Debug + Clone>() -> impl ::winnow::Parser<
        ::winnow_grammar::ParseInput<'a, S>,
        Vec<Value>,
        ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
    > {
        move |
            input: &mut ::winnow_grammar::ParseInput<'a, S>,
        | -> ::winnow::Result<
            Vec<Value>,
            ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
        > {
            let _ = WS(input)?;
            let result = parse_list_content_inner(input)?;
            let _ = WS(input)?;
            ::winnow::combinator::eof.parse_next(input)?;
            Ok(result)
        }
    }
}
extern crate test;
#[rustc_test_marker = "test_mixed_values"]
#[doc(hidden)]
pub const test_mixed_values: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_mixed_values"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "winnow-grammar/tests/comprehensive.rs",
        start_line: 31usize,
        start_col: 4usize,
        end_line: 31usize,
        end_col: 21usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_mixed_values()),
    ),
};
fn test_mixed_values() {
    Comprehensive::parse_value().parse_test("123").assert_success_is(Value::Int(123));
    Comprehensive::parse_value()
        .parse_test("123.456")
        .assert_success_with(|v| match v {
            Value::Float(f) => {
                if !((f - 123.456).abs() < 1e-6) {
                    ::core::panicking::panic(
                        "assertion failed: (f - 123.456).abs() < 1e-6",
                    )
                }
            }
            _ => {
                ::core::panicking::panic_fmt(
                    format_args!("Expected Float for 123.456, got {0:?}", v),
                );
            }
        });
    Comprehensive::parse_value()
        .parse_test("123e2")
        .assert_success_with(|v| match v {
            Value::Float(f) => {
                if !((f - 12300.0).abs() < 1e-6) {
                    ::core::panicking::panic(
                        "assertion failed: (f - 12300.0).abs() < 1e-6",
                    )
                }
            }
            _ => {
                ::core::panicking::panic_fmt(
                    format_args!("Expected Float for 123e2, got {0:?}", v),
                );
            }
        });
    Comprehensive::parse_value()
        .parse_test("\"hello\"")
        .assert_success_is(Value::String("hello".to_string()));
    Comprehensive::parse_value()
        .parse_test("[1, \"two\", 3.0]")
        .assert_success_with(|v| {
            if let Value::List(l) = v {
                match (&l.len(), &3) {
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
                match (&l[0], &Value::Int(1)) {
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
                match (&l[1], &Value::String("two".to_string())) {
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
                if let Value::Float(f) = l[2] {
                    if !((f - 3.0).abs() < 1e-6) {
                        ::core::panicking::panic(
                            "assertion failed: (f - 3.0).abs() < 1e-6",
                        )
                    }
                } else {
                    {
                        ::core::panicking::panic_fmt(
                            format_args!("Expected float at index 2, got {0:?}", l[2]),
                        );
                    };
                }
            } else {
                {
                    ::core::panicking::panic_fmt(format_args!("Expected list"));
                };
            }
        });
}
#[allow(non_snake_case)]
pub mod GenericReturn {
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
    fn parse_optional_int_inner<'a, S: std::fmt::Debug + Clone>(
        input: &mut ::winnow_grammar::ParseInput<'a, S>,
    ) -> ::winnow::Result<
        Option<i32>,
        ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
    > {
        use ::winnow::Parser;
        let mut parser = (|
            input: &mut ::winnow_grammar::ParseInput<'a, S>,
        | -> ::winnow::Result<
            Option<i32>,
            ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
        > {
            use ::winnow::prelude::*;
            alt((
                    |
                        input: &mut ::winnow_grammar::ParseInput<'a, S>,
                    | -> ::winnow::Result<_> {
                        let _ = WS(input)?;
                        let i = ::winnow::ascii::dec_int::<
                            ::winnow_grammar::ParseInput<'a, S>,
                            i32,
                            ::winnow::error::InputError<
                                ::winnow_grammar::ParseInput<'a, S>,
                            >,
                        >
                            .parse_next(input)?;
                        Ok({ Some(i) })
                    },
                    |
                        input: &mut ::winnow_grammar::ParseInput<'a, S>,
                    | -> ::winnow::Result<_> {
                        let _ = WS(input)?;
                        let _ = literal("none")
                            .context(
                                ::winnow::error::StrContext::Expected(
                                    ::winnow::error::StrContextValue::StringLiteral("none"),
                                ),
                            )
                            .parse_next(input)?;
                        Ok({ None })
                    },
                ))
                .parse_next(input)
        })
            .context(::winnow::error::StrContext::Label("optional_int"));
        { parser.parse_next(input) }
    }
    pub fn parse_optional_int<'a, S: std::fmt::Debug + Clone>() -> impl ::winnow::Parser<
        ::winnow_grammar::ParseInput<'a, S>,
        Option<i32>,
        ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
    > {
        move |
            input: &mut ::winnow_grammar::ParseInput<'a, S>,
        | -> ::winnow::Result<
            Option<i32>,
            ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
        > {
            let _ = WS(input)?;
            let result = parse_optional_int_inner(input)?;
            let _ = WS(input)?;
            ::winnow::combinator::eof.parse_next(input)?;
            Ok(result)
        }
    }
}
extern crate test;
#[rustc_test_marker = "test_generic_return"]
#[doc(hidden)]
pub const test_generic_return: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_generic_return"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "winnow-grammar/tests/comprehensive.rs",
        start_line: 82usize,
        start_col: 4usize,
        end_line: 82usize,
        end_col: 23usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_generic_return()),
    ),
};
fn test_generic_return() {
    GenericReturn::parse_optional_int().parse_test("42").assert_success_is(Some(42));
    GenericReturn::parse_optional_int().parse_test("none").assert_success_is(None);
}
#[allow(non_snake_case)]
pub mod NumFormats {
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
    fn parse_hex_inner<'a, S: std::fmt::Debug + Clone>(
        input: &mut ::winnow_grammar::ParseInput<'a, S>,
    ) -> ::winnow::Result<
        u32,
        ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
    > {
        use ::winnow::Parser;
        let mut parser = (|
            input: &mut ::winnow_grammar::ParseInput<'a, S>,
        | -> ::winnow::Result<
            u32,
            ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
        > {
            use ::winnow::prelude::*;
            {
                let _ = WS(input)?;
                let _ = literal("0x")
                    .context(
                        ::winnow::error::StrContext::Expected(
                            ::winnow::error::StrContextValue::StringLiteral("0x"),
                        ),
                    )
                    .parse_next(input)?;
                let _ = WS(input)?;
                let h = ::winnow::ascii::hex_digit1::<
                    ::winnow_grammar::ParseInput<'a, S>,
                    ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
                >
                    .parse_next(input)?;
                Ok(u32::from_str_radix(&h, 16).unwrap())
            }
        })
            .context(::winnow::error::StrContext::Label("hex"));
        { parser.parse_next(input) }
    }
    pub fn parse_hex<'a, S: std::fmt::Debug + Clone>() -> impl ::winnow::Parser<
        ::winnow_grammar::ParseInput<'a, S>,
        u32,
        ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
    > {
        move |
            input: &mut ::winnow_grammar::ParseInput<'a, S>,
        | -> ::winnow::Result<
            u32,
            ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
        > {
            let _ = WS(input)?;
            let result = parse_hex_inner(input)?;
            let _ = WS(input)?;
            ::winnow::combinator::eof.parse_next(input)?;
            Ok(result)
        }
    }
    #[allow(dead_code)]
    fn parse_oct_inner<'a, S: std::fmt::Debug + Clone>(
        input: &mut ::winnow_grammar::ParseInput<'a, S>,
    ) -> ::winnow::Result<
        u32,
        ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
    > {
        use ::winnow::Parser;
        let mut parser = (|
            input: &mut ::winnow_grammar::ParseInput<'a, S>,
        | -> ::winnow::Result<
            u32,
            ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
        > {
            use ::winnow::prelude::*;
            {
                let _ = WS(input)?;
                let _ = literal("0o")
                    .context(
                        ::winnow::error::StrContext::Expected(
                            ::winnow::error::StrContextValue::StringLiteral("0o"),
                        ),
                    )
                    .parse_next(input)?;
                let _ = WS(input)?;
                let o = ::winnow::ascii::oct_digit1::<
                    ::winnow_grammar::ParseInput<'a, S>,
                    ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
                >
                    .parse_next(input)?;
                Ok(u32::from_str_radix(&o, 8).unwrap())
            }
        })
            .context(::winnow::error::StrContext::Label("oct"));
        { parser.parse_next(input) }
    }
    pub fn parse_oct<'a, S: std::fmt::Debug + Clone>() -> impl ::winnow::Parser<
        ::winnow_grammar::ParseInput<'a, S>,
        u32,
        ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
    > {
        move |
            input: &mut ::winnow_grammar::ParseInput<'a, S>,
        | -> ::winnow::Result<
            u32,
            ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
        > {
            let _ = WS(input)?;
            let result = parse_oct_inner(input)?;
            let _ = WS(input)?;
            ::winnow::combinator::eof.parse_next(input)?;
            Ok(result)
        }
    }
    #[allow(dead_code)]
    fn parse_bin_inner<'a, S: std::fmt::Debug + Clone>(
        input: &mut ::winnow_grammar::ParseInput<'a, S>,
    ) -> ::winnow::Result<
        u32,
        ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
    > {
        use ::winnow::Parser;
        let mut parser = (|
            input: &mut ::winnow_grammar::ParseInput<'a, S>,
        | -> ::winnow::Result<
            u32,
            ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
        > {
            use ::winnow::prelude::*;
            {
                let _ = WS(input)?;
                let _ = literal("0b")
                    .context(
                        ::winnow::error::StrContext::Expected(
                            ::winnow::error::StrContextValue::StringLiteral("0b"),
                        ),
                    )
                    .parse_next(input)?;
                let _ = WS(input)?;
                let b = ::winnow::token::take_while(1.., |c| c == '0' || c == '1')
                    .parse_next(input)?;
                Ok(u32::from_str_radix(&b, 2).unwrap())
            }
        })
            .context(::winnow::error::StrContext::Label("bin"));
        { parser.parse_next(input) }
    }
    pub fn parse_bin<'a, S: std::fmt::Debug + Clone>() -> impl ::winnow::Parser<
        ::winnow_grammar::ParseInput<'a, S>,
        u32,
        ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
    > {
        move |
            input: &mut ::winnow_grammar::ParseInput<'a, S>,
        | -> ::winnow::Result<
            u32,
            ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
        > {
            let _ = WS(input)?;
            let result = parse_bin_inner(input)?;
            let _ = WS(input)?;
            ::winnow::combinator::eof.parse_next(input)?;
            Ok(result)
        }
    }
}
extern crate test;
#[rustc_test_marker = "test_num_formats"]
#[doc(hidden)]
pub const test_num_formats: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_num_formats"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "winnow-grammar/tests/comprehensive.rs",
        start_line: 107usize,
        start_col: 4usize,
        end_line: 107usize,
        end_col: 20usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_num_formats()),
    ),
};
fn test_num_formats() {
    NumFormats::parse_hex().parse_test("0x1A").assert_success_is(26);
    NumFormats::parse_oct().parse_test("0o12").assert_success_is(10);
    NumFormats::parse_bin().parse_test("0b1010").assert_success_is(10);
}
#[allow(non_snake_case)]
pub mod LargeInt {
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
    fn parse_int64_inner<'a, S: std::fmt::Debug + Clone>(
        input: &mut ::winnow_grammar::ParseInput<'a, S>,
    ) -> ::winnow::Result<
        i64,
        ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
    > {
        use ::winnow::Parser;
        let mut parser = (|
            input: &mut ::winnow_grammar::ParseInput<'a, S>,
        | -> ::winnow::Result<
            i64,
            ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
        > {
            use ::winnow::prelude::*;
            {
                let _ = WS(input)?;
                let s = ::winnow::ascii::digit1::<
                    ::winnow_grammar::ParseInput<'a, S>,
                    ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
                >
                    .parse_next(input)?;
                Ok(s.parse().unwrap())
            }
        })
            .context(::winnow::error::StrContext::Label("int64"));
        { parser.parse_next(input) }
    }
    pub fn parse_int64<'a, S: std::fmt::Debug + Clone>() -> impl ::winnow::Parser<
        ::winnow_grammar::ParseInput<'a, S>,
        i64,
        ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
    > {
        move |
            input: &mut ::winnow_grammar::ParseInput<'a, S>,
        | -> ::winnow::Result<
            i64,
            ::winnow::error::InputError<::winnow_grammar::ParseInput<'a, S>>,
        > {
            let _ = WS(input)?;
            let result = parse_int64_inner(input)?;
            let _ = WS(input)?;
            ::winnow::combinator::eof.parse_next(input)?;
            Ok(result)
        }
    }
}
extern crate test;
#[rustc_test_marker = "test_int64"]
#[doc(hidden)]
pub const test_int64: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_int64"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "winnow-grammar/tests/comprehensive.rs",
        start_line: 130usize,
        start_col: 4usize,
        end_line: 130usize,
        end_col: 14usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_int64()),
    ),
};
fn test_int64() {
    LargeInt::parse_int64()
        .parse_test("9223372036854775807")
        .assert_success_is(i64::MAX);
}
#[rustc_main]
#[coverage(off)]
#[doc(hidden)]
pub fn main() -> () {
    extern crate test;
    test::test_main_static(
        &[&test_generic_return, &test_int64, &test_mixed_values, &test_num_formats],
    )
}
