use syn_grammar::grammar;
use syn_grammar::testing::Testable;

mod err_test_1 {
    use super::*;
    grammar! {
        grammar err_test_1 {
            pub rule main -> () = call!(deepest_err);
            rule deepest_err -> () = "a" "b" "c" -> { () };
        }
    }
}

mod digit_test_1 {
    use super::*;
    grammar! {
        grammar digit_test_1 {
            pub rule main -> () = l:letter+ d:digit+ -> { () };
            rule letter -> () = "a" | "b" | "c" -> { () };
            rule digit -> () = "0" | "1" | "2" -> { () };
        }
    }
}

#[test]
fn test_deepest_error_wins() {
    err_test_1::parse_main
        .parse_str("a b d")
        .test()
        .assert_error_contains(0, "expected `c`");

    digit_test_1::parse_main
        .parse_str("a b 1 c")
        .test()
        .assert_error_contains(0, "expected `0`, `1`, or `2`");
}

mod prio_test {
    use super::*;
    grammar! {
        grammar prio_test {
            pub rule main -> () = d:deep | s:shallow -> { () };
            rule deep -> () = "a" "b" "c" -> { () };
            rule shallow -> () = "d" "e" -> { () };
        }
    }
}

#[test]
fn test_deep_vs_shallow() {
    prio_test::parse_main
        .parse_str("a b d")
        .test()
        .assert_error_contains(0, "expected `c`");
}

mod rule_name_test {
    use super::*;
    grammar! {
        grammar rule_name_test {
            pub rule main -> () = a:inner_rule -> { () };
            rule inner_rule -> () = "a" "b" -> { () };
        }
    }
}

#[test]
fn test_rule_name_in_error_message() {
    rule_name_test::parse_main
        .parse_str("a c")
        .test()
        .assert_error_contains(0, "in rule `inner_rule`");
}
