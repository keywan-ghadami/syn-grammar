use syn::parse::Parser;
use syn_grammar::grammar;
use syn_grammar::testing::Testable;

// Top level grammars

grammar! {
    grammar action_block_test {
        pub rule main -> i32 = "a" -> {
            let a = 1;
            let b = 2;
            a + b
        }
    }
}

grammar! {
    grammar builtins_test {
        pub rule ident -> String = i:ident -> { i.to_string() }
        pub rule end -> () = eof -> { () }
    }
}

grammar! {
    grammar repetition_test {
        pub rule star -> Vec<i32> = ("a" -> { 1 })* -> { list }
        pub rule plus -> Vec<i32> = ("a" -> { 1 })+ -> { list }
        pub rule optional -> Option<i32> = ("a" -> { 1 })? -> { opt }
    }
}

grammar! {
    grammar nested_repetition_test {
        pub rule main -> Vec<Vec<i32>> =
            (
                ("a" -> { 1 })+
                ("," -> {})?
            )* -> { list }
    }
}

grammar! {
    grammar cut_test {
        pub rule main -> i32 =
            "a" => "b" -> { 1 }
            | "a" "c" -> { 2 }
    }
}

grammar! {
    grammar kw_test {
        pub rule main -> i32 =
            "fn" -> { 1 }
            | "struct" -> { 2 }
            | i:ident -> { 3 }
    }
}

grammar! {
    grammar seq_test {
        pub rule main -> (i32, i32) = a:("a" -> {1}) b:("b" -> {2}) -> { (a, b) }
    }
}

grammar! {
    grammar epsilon_test {
        pub rule main -> i32 =
            i:inner? -> { i.unwrap_or(0) }

        rule inner -> i32 = "a" -> { 1 }
    }
}

grammar! {
    grammar args {
        pub rule main -> i32 = val<>(1) -> { val }
        rule val(x: i32) -> i32 = "a" -> { x + 1 }
    }
}

grammar! {
    grammar multi_args {
        pub rule main -> i32 = call<>(1, 2) -> { call }
        rule call(x: i32, y: i32) -> i32 = "a" -> { x + y }
    }
}

grammar! {
    grammar types {
        use std::rc::Rc;
        use std::cell::RefCell;
        pub rule main -> Rc<RefCell<i32>> =
            item:i32 -> {
                Rc::new(RefCell::new(item))
            }
    }
}

grammar! {
    grammar cut_rep {
        pub rule main -> () = ("a" => "b")* "c" -> { () }
    }
}

grammar! {
    grammar prio {
        pub rule main -> i32 =
            "a" "b" -> { 1 }
            | "a" -> { 2 }
    }
}

grammar! {
    grammar use_stmt {
        use std::rc::Rc;
        pub rule main -> Rc<i32> = i:i32 -> { Rc::new(i) }
    }
}

grammar! {
    grammar multi_token {
        pub rule main -> () =
            "?." -> { () }
    }
}

grammar! {
    grammar attributes {
        /// Doc comment
        #[allow(unused)]
        pub rule main -> () = "a" -> { () }
    }
}

grammar! {
    grammar plus_validation {
        pub rule main -> Vec<()> = ("a" -> {()} | "b" -> {()})+ -> { list }
    }
}

grammar! {
    grammar rust_stuff {
        pub rule ty -> String = "i32" -> { "i32".to_string() }
        pub rule block -> i32 = "{" "}" -> { 1 }
    }
}

grammar! {
    grammar fail_test_1 {
        pub rule main -> i32 =
            "DEBUG" e:expr -> { e }
            | e:expr -> { e }

        rule expr -> i32 =
            i:i32 -> { i }
            | "a" -> { 1 }
    }
}

grammar! {
    grammar fail_test_2 {
        pub rule main -> i32 =
            e:expr "DEBUG" -> { e }
            | e:expr -> { e }

        rule expr -> i32 =
            i:i32 -> { i }
            | "a" -> { 1 }
    }
}

#[test]
fn test_action_block_statements() {
    action_block_test::parse_main
        .parse_str("a")
        .test()
        .assert_success_is(3);
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
    repetition_test::parse_plus
        .parse_str("")
        .test()
        .assert_is_err();
    repetition_test::parse_optional
        .parse_str("")
        .test()
        .assert_success_is(None);
    repetition_test::parse_optional
        .parse_str("a")
        .test()
        .assert_success_is(Some(1));
}

#[test]
fn test_nested_repetition_complex() {
    nested_repetition_test::parse_main
        .parse_str("a, a a, a")
        .test()
        .assert_success_is(vec![vec![1], vec![1, 1], vec![1]]);
}

#[test]
fn test_cut_operator() {
    cut_test::parse_main
        .parse_str("a b")
        .test()
        .assert_success_is(1);
    let res = cut_test::parse_main.parse_str("a c").test();
    res.assert_is_err();
}

#[test]
fn test_keywords_vs_idents() {
    kw_test::parse_main
        .parse_str("fn")
        .test()
        .assert_success_is(1);
    kw_test::parse_main
        .parse_str("struct")
        .test()
        .assert_success_is(2);
    kw_test::parse_main
        .parse_str("a")
        .test()
        .assert_success_is(3);
    kw_test::parse_main
        .parse_str("fnx")
        .test()
        .assert_success_is(3);
}

#[test]
fn test_basic_sequence() {
    seq_test::parse_main
        .parse_str("a b")
        .test()
        .assert_success_is((1, 2));
}

#[test]
fn test_epsilon_alternative() {
    epsilon_test::parse_main
        .parse_str("a")
        .test()
        .assert_success_is(1);
    epsilon_test::parse_main
        .parse_str("")
        .test()
        .assert_success_is(0);
}

#[test]
fn test_rule_arguments() {
    args::parse_main.parse_str("a").test().assert_success_is(2);
}

#[test]
fn test_multiple_arguments() {
    multi_args::parse_main
        .parse_str("a")
        .test()
        .assert_success_is(3);
}

#[test]
fn test_complex_return_types() {
    let result = types::parse_main.parse_str("123").test();
    assert_eq!(*result.get_success_value().borrow(), 123);
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

#[test]
fn test_backtracking_priority() {
    prio::parse_main
        .parse_str("a b")
        .test()
        .assert_success_is(1);
    prio::parse_main.parse_str("a").test().assert_success_is(2);
}

#[test]
fn test_use_statements() {
    let result = use_stmt::parse_main.parse_str("123").test();
    assert_eq!(*result.get_success_value(), 123);
}

#[test]
fn test_multi_token_literals() {
    multi_token::parse_main
        .parse_str("?.")
        .test()
        .assert_success_is(());
}

#[test]
fn test_attributes_on_rules() {
    attributes::parse_main
        .parse_str("a")
        .test()
        .assert_success_is(());
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

#[test]
fn test_fail_builtin_first() {
    fail_test_1::parse_main
        .parse_str("1")
        .test()
        .assert_success_is(1);
    fail_test_1::parse_main
        .parse_str("DEBUG 1")
        .test()
        .assert_success_is(1);
}

#[test]
fn test_fail_builtin_last() {
    fail_test_2::parse_main
        .parse_str("1")
        .test()
        .assert_success_is(1);
    fail_test_2::parse_main
        .parse_str("1 DEBUG")
        .test()
        .assert_success_is(1);
}
