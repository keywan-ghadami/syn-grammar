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
    grammar kw_test {
        pub rule main -> i32 =
            "fn" -> { 1 }
            | "struct" -> { 2 }
            | i:ident -> { 3 }
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
    grammar rust_features {
        pub rule type_parser -> syn::Type = t:rust_type -> { t }
        pub rule block_parser -> syn::Block = b:rust_block -> { b }
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
fn test_complex_return_types() {
    let result = types::parse_main.parse_str("123").test();
    assert_eq!(*result.assert_success().borrow(), 123);
}

#[test]
fn test_use_statements() {
    let result = use_stmt::parse_main.parse_str("123").test();
    assert_eq!(*result.assert_success(), 123);
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
fn test_rust_types_and_blocks() {
    rust_features::parse_type_parser
        .parse_str("Vec<i32>")
        .test()
        .assert_success();

    rust_features::parse_block_parser
        .parse_str("{ let x = 1; }")
        .test()
        .assert_success();
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
