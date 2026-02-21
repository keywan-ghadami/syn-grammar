use syn::parse::Parser;
use syn_grammar::grammar;
use syn_grammar::testing::Testable;

pub mod basemod {
    use syn_grammar::grammar;
    grammar! {
        grammar base {
            pub rule digit -> i32 = "a" -> { 0 }
        }
    }
}

pub mod derived {
    use crate::base_rules;
    use syn_grammar::grammar;
    grammar! {
        include base_rules as b;
        grammar derived {
            pub rule main -> i32 = d:b::digit -> { d }
        }
    }
}

#[test]
fn test_composition_basic() {
    derived::derived::parse_main
        .parse_str("a")
        .test()
        .assert_success_is(0);
}

pub mod g1 {
    use syn_grammar::grammar;
    grammar! {
        grammar G1 {
            pub rule value -> i32 = "a" -> { 1 }
        }
    }
}

pub mod g2 {
    use syn_grammar::grammar;
    grammar! {
        grammar G2 {
            pub rule value -> i32 = "b" -> { 2 }
        }
    }
}

mod combined {
    use crate::G1_rules;
    use crate::G2_rules;
    use syn_grammar::grammar;
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
