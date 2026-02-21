use syn_grammar::grammar;
use syn_grammar::testing::Testable;

mod action_block_test {
    use super::*;
    grammar! {
        grammar action_block_test {
            pub rule main -> i32 = "a" -> {
                let a = 1;
                let b = 2;
                a + b
            };
        }
    }
}

#[test]
fn test_action_block_statements() {
    action_block_test::parse_main
        .parse_str("a")
        .test()
        .assert_success_is(3);
}

mod builtins_test {
    use super::*;
    grammar! {
        grammar builtins_test {
            pub rule ident -> String = i:IDENT -> { i.to_string() };
            pub rule end -> () = EOI -> { () };
        }
    }
}

#[test]
fn test_builtins() {
    builtins_test::parse_ident
        .parse_str("abc")
        .test()
        .assert_success_is("abc".to_string());
    builtins_test::parse_end
        .parse_str("")
        .test()
        .assert_success_is(());
}

mod repetition_test {
    use super::*;
    grammar! {
        grammar repetition_test {
            pub rule star -> Vec<i32> = ( "a" -> { 1 } )* -> { list };
            pub rule plus -> Vec<i32> = ( "a" -> { 1 } )+ -> { list };
            pub rule optional -> Option<i32> = ( "a" -> { 1 } )? -> { opt };
        }
    }
}

#[test]
fn test_repetition() {
    repetition_test::parse_star
        .parse_str("")
        .test()
        .assert_success_is(vec![]);
    repetition_test::parse_star
        .parse_str("a a")
        .test()
        .assert_success_is(vec![1, 1]);

    repetition_test::parse_plus
        .parse_str("a")
        .test()
        .assert_success_is(vec![1]);
    repetition_test::parse_plus
        .parse_str("a a")
        .test()
        .assert_success_is(vec![1, 1]);
    repetition_test::parse_plus.parse_str("").test().assert_is_err();

    repetition_test::parse_optional
        .parse_str("")
        .test()
        .assert_success_is(None);
    repetition_test::parse_optional
        .parse_str("a")
        .test()
        .assert_success_is(Some(1));
}

mod nested_repetition_test {
    use super::*;
    grammar! {
        grammar nested_repetition_test {
            pub rule main -> Vec<Vec<i32>> =
                (
                    ( "a" -> { 1 } )+
                    ( "," -> {} )?
                )* -> { list };
        }
    }
}

#[test]
fn test_nested_repetition_complex() {
    nested_repetition_test::parse_main
        .parse_str("a, a a, a")
        .test()
        .assert_success_is(vec![vec![1], vec![1, 1], vec![1]]);
}

mod cut_test {
    use super::*;
    grammar! {
        grammar cut_test {
            pub rule main -> i32 =
                "a" => "b" -> { 1 }
                | "a" "c" -> { 2 };
        }
    }
}

#[test]
fn test_cut_operator() {
    cut_test::parse_main
        .parse_str("a b")
        .test()
        .assert_success_is(1);
    let res = cut_test::parse_main.parse_str("a c").test();
    res.assert_is_err();
    assert!(!res.get_err_str().contains("expected `c`"));
}

mod expr_test {
    use super::*;
    grammar! {
        grammar expr_test {
            pub rule main -> i32 = e:expr(0) EOI -> { e };

            rule expr(min_prec: u8) -> i32 =
                l:expr_base -> {
                    let mut l = l;
                    loop {
                        if peek! { "+" } && 1 >= min_prec {
                            // Infix
                            expect! { "+" };
                            let r = call!(expr(1));
                            l = l + r;
                        } else if peek! { "*" } && 2 >= min_prec {
                            expect! { "*" };
                            let r = call!(expr(2));
                            l = l * r;
                        } else {
                            break;
                        }
                    }
                    l
                };

            rule expr_base -> i32 =
                i:INT -> { i.parse().unwrap() }
                | "(" e:expr(0) ")" -> { e };
        }
    }
}

#[test]
fn test_left_recursion() {
    // Left-recursion is supported through Pratt parsing.
    // A rule is a Pratt-style rule if it takes a `min_prec` argument.
    // Inside a Pratt rule, ` call!` is a Pratt-style recursive call.
    fn check(str: &str, val: i32) {
        expr_test::parse_main
            .parse_str(str)
            .test()
            .assert_success_is(val);
    }

    check("1", 1);
    check("1 + 2", 3);
    check("1 * 2", 2);
    check("1 + 2 * 3", 7);
    check("(1 + 2) * 3", 9);
}

mod kw_test {
    use super::*;
    grammar! {
        grammar kw_test {
            pub rule main -> i32 =
                "fn" -> { 1 }
                | "struct" -> { 2 }
                | i:IDENT -> { 3 };
        }
    }
}

#[test]
fn test_keywords_vs_idents() {
    fn check(s: &str, v: i32) {
        kw_test::parse_main
            .parse_str(s)
            .test()
            .assert_success_is(v);
    }

    check("fn", 1);
    check("struct", 2);
    check("a", 3);
    check("fnx", 3);
}

mod seq_test {
    use super::*;
    grammar! {
        grammar seq_test {
            pub rule main -> (i32, i32) = a:("a" -> {1}) b:("b" -> {2}) -> { (a, b) };
        }
    }
}

#[test]
fn test_basic_sequence() {
    seq_test::parse_main
        .parse_str("a b")
        .test()
        .assert_success_is((1, 2));
}

mod epsilon_test {
    use super::*;
    grammar! {
        grammar epsilon_test {
            pub rule main -> i32 =
                i:inner? -> { i.unwrap_or(0) };

            rule inner -> i32 = "a" -> { 1 };
        }
    }
}

#[test]
fn test_epsilon_alternative() {
    epsilon_test::parse_main
        .parse_str("a")
        .test()
        .assert_success_is(1);
    epsilon_test::parse_main.parse_str("").test().assert_success_is(0);
}

mod args {
    use super::*;
    grammar! {
        grammar args {
            pub rule main -> i32 = call!(val(1));
            rule val(x: i32) -> i32 = "a" -> { x + 1 };
        }
    }
}

#[test]
fn test_rule_arguments() {
    args::parse_main
        .parse_str("a")
        .test()
        .assert_success_is(2);
}

mod multi_args {
    use super::*;
    grammar! {
        grammar multi_args {
            pub rule main -> i32 = call!(call(1, 2));
            rule call(x: i32, y: i32) -> i32 = "a" -> { x + y };
        }
    }
}

#[test]
fn test_multiple_arguments() {
    multi_args::parse_main
        .parse_str("a")
        .test()
        .assert_success_is(3);
}

mod types {
    use super::*;
    use std::rc::Rc;
    use std::cell::RefCell;
    grammar! {
        grammar types {
            pub rule main -> Rc<RefCell<i32>> =
                item:INT -> {
                    Rc::new(RefCell::new(item.parse().unwrap()))
                };
        }
    }
}

#[test]
fn test_complex_return_types() {
    let result = types::parse_main.parse_str("123").test();
    assert_eq!(*result.get_success_value().borrow(), 123);
}

mod cut_rep {
    use super::*;
    grammar! {
        grammar cut_rep {
            pub rule main -> () = ("a" => "b")* "c" -> { () };
        }
    }
}

#[test]
fn test_cut_in_repetition() {
    cut_rep::parse_main
        .parse_str("a b a b c")
        .test()
        .assert_success_is(());
    cut_rep::parse_main
        .parse_str("a c")
        .test()
        .assert_error_contains(0, "expected `b`");
}

mod prio {
    use super::*;
    grammar! {
        grammar prio {
            pub rule main -> i32 =
                "a" "b" -> { 1 }
                | "a" -> { 2 };
        }
    }
}

#[test]
fn test_backtracking_priority() {
    // Since "a" "b" is a longer match, it should be preferred.
    prio::parse_main
        .parse_str("a b")
        .test()
        .assert_success_is(1);
    prio::parse_main
        .parse_str("a")
        .test()
        .assert_success_is(2);
}

mod use_stmt {
    use super::*;
    grammar! {
        grammar use_stmt {
            use std::rc::Rc;
            pub rule main -> Rc<i32> = i:INT -> { Rc::new(i.parse().unwrap()) };
        }
    }
}

#[test]
fn test_use_statements() {
    let result = use_stmt::parse_main.parse_str("123").test();
    assert_eq!(*result.get_success_value(), 123);
}

#[derive(Debug, PartialEq, Eq)]
pub enum Expr {
    Ident(String),
    Field(Box<Expr>, String),
}

mod field_access {
    use super::*;
    grammar! {
        grammar field_access {
            pub rule main -> super::Expr = l:expr_base -> {
                let mut l = l;
                loop {
                    if peek! { "." } {
                        expect! { "." };
                        let r = expect! { IDENT };
                        l = super::Expr::Field(Box::new(l), r.to_string());
                    } else {
                        break;
                    }
                }
                l
            };

            rule expr_base -> super::Expr = i:IDENT -> { super::Expr::Ident(i.to_string()) };
        }
    }
}

#[test]
fn test_left_recursion_field_access() {
    field_access::parse_main
        .parse_str("a.b.c")
        .test()
        .assert_success_is(Expr::Field(
            Box::new(Expr::Field(
                Box::new(Expr::Ident("a".to_string())),
                "b".to_string(),
            )),
            "c".to_string(),
        ));
}

mod multi_token {
    use super::*;
    grammar! {
        grammar multi_token {
            pub rule main -> () =
                "?." -> { () };
        }
    }
}

#[test]
fn test_multi_token_literals() {
    multi_token::parse_main
        .parse_str("?.")
        .test()
        .assert_success_is(());
}

mod extended_literals {
    use super::*;
    grammar! {
        grammar extended_literals {
            pub rule main -> (char, i32, f32) =
                a:'a' b:1 c:1.2 -> { (a, b, c) };
        }
    }
}

#[test]
fn test_extended_literals() {
    extended_literals::parse_main
        .parse_str("'a' 1 1.2")
        .test()
        .assert_success_is(('a', 1, 1.2));
}

mod attributes {
    use super::*;
    grammar! {
        grammar attributes {
            /// Doc comment
            #[allow(unused)]
            pub rule main -> () = "a" -> { () };
        }
    }
}

#[test]
fn test_attributes_on_rules() {
    attributes::parse_main
        .parse_str("a")
        .test()
        .assert_success_is(());
}

mod plus_validation {
    use super::*;
    grammar! {
        grammar plus_validation {
            pub rule main -> Vec<()> = ("a" | "b")+ -> { list };
        }
    }
}

#[test]
fn test_plus_operator_validation() {
    plus_validation::parse_main
        .parse_str("")
        .test()
        .assert_is_err();
    plus_validation::parse_main
        .parse_str("a b a")
        .test()
        .assert_success_is(vec![(), (), ()]);
}

mod math {
    use super::*;
    grammar! {
        grammar math {
            pub rule main -> i32 = l:expr_base -> {
                let mut l = l;
                loop {
                    if peek! { "+" } {
                        expect! { "+" };
                        let r = call!(main());
                        l += r;
                    } else {
                        break;
                    }
                }
                l
            };
            rule expr_base -> i32 =
                i:INT -> { i.parse().unwrap() }
                | "(" e:main() ")" -> { e };
        }
    }
}

#[test]
fn test_math_expression() {
    fn check(s: &str, v: i32) {
        math::parse_main.parse_str(s).test().assert_success_is(v);
    }

    check("1", 1);
    check("1 + 2", 3);
    check("1 + 2 + 3", 6);
    check("(1 + 2) + 3", 6);
}

mod rust_stuff {
    use super::*;
    grammar! {
        grammar rust_stuff {
            pub rule ty -> String = "i32" -> { "i32".to_string() };
            pub rule block -> i32 = "{" "}" -> { 1 };
        }
    }
}

#[test]
fn test_rust_types_and_blocks() {
    rust_stuff::parse_ty
        .parse_str("i32")
        .test()
        .assert_success_is("i32".to_string());
    rust_stuff::parse_block
        .parse_str("{ }")
        .test()
        .assert_success_is(1);
}

mod fail_test_1 {
    use super::*;
    grammar! {
        grammar fail_test_1 {
            pub rule main -> i32 =
                "DEBUG" e:expr -> { e }
                | e:expr -> { e };

            rule expr -> i32 =
                i:INT -> { i.parse().unwrap() }
                | "a" -> { 1 };
        }
    }
}

#[test]
fn test_fail_builtin_first() {
    fail_test_1::parse_main.parse_str("1").test().assert_success_is(1);
    fail_test_1::parse_main.parse_str("DEBUG 1").test().assert_success_is(1);
}

mod fail_test_2 {
    use super::*;
    grammar! {
        grammar fail_test_2 {
            pub rule main -> i32 =
                e:expr "DEBUG" -> { e }
                | e:expr -> { e };

            rule expr -> i32 =
                i:INT -> { i.parse().unwrap() }
                | "a" -> { 1 };
        }
    }
}

#[test]
fn test_fail_builtin_last() {
    fail_test_2::parse_main.parse_str("1").test().assert_success_is(1);
    fail_test_2::parse_str("1 DEBUG").test().assert_success_is(1);
}

mod gap {
    use super::*;
    grammar! {
        grammar gap {
            pub rule main -> Vec<i32> = (i:INT -> { i.parse().unwrap() })* gap!(INT);
        }
    }
}

#[test]
fn test_gap_detection() {
    gap::parse_main.parse_str("1 2 3").test().assert_success_is(vec![1, 2, 3]);
    gap::parse_main.parse_str("1 2 3 4").test().assert_success_is(vec![1, 2, 3]);
}
