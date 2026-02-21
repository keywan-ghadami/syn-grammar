use syn_grammar::grammar;
use syn_grammar::testing::Testable;

mod base {
    use super::*;
    grammar! {
        grammar Base {
            pub rule digit -> i32 = "a" -> { 0 }
        }
    }
}

mod derived {
    use super::*;
    use super::base::Base_rules;
    grammar! {
        include Base_rules as b;
        grammar Derived {
            pub rule main -> i32 = d:b::digit -> { d }
        }
    }
}

#[test]
fn test_composition_basic() {
    derived::Derived::parse_main
        .parse_str("a")
        .test()
        .assert_success_is(0);
}

mod g1 {
    use super::*;
    grammar! {
        grammar G1 {
            pub rule value -> i32 = "a" -> { 1 }
        }
    }
}

mod g2 {
    use super::*;
    grammar! {
        grammar G2 {
            pub rule value -> i32 = "b" -> { 2 }
        }
    }
}

mod combined {
    use super::*;
    use super::g1::G1_rules;
    use super::g2::G2_rules;
    grammar! {
        include G1_rules as g1;
        include G2_rules as g2;

        grammar Combined {
            pub rule main -> (i32, i32) =
                v1:g1::value v2:g2::value -> { (v1, v2) }
        }
    }
}

#[test]
fn test_composition_mangling() {
    combined::Combined::parse_main
        .parse_str("a b")
        .test()
        .assert_success_is((1, 2));
}
