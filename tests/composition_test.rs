use syn_grammar::grammar;
use syn_grammar::testing::Testable;

#[test]
fn test_composition_basic() {
    {
        grammar! {
            grammar Base {
                pub rule digit -> i32 = "a" -> { 0 }
            }
        }
    }

    {
        grammar! {
            include Base_rules as b;
            grammar Derived {
                pub rule main -> i32 = d:b::digit -> { d }
            }
        }

        Derived::parse_main
            .parse_str("a")
            .test()
            .assert_success_is(0);
    }
}

#[test]
fn test_composition_mangling() {
    {
        grammar! {
            grammar G1 {
                pub rule value -> i32 = "a" -> { 1 }
            }
        }
    }

    {
        grammar! {
            grammar G2 {
                pub rule value -> i32 = "b" -> { 2 }
            }
        }
    }

    {
        grammar! {
            include G1_rules as g1;
            include G2_rules as g2;

            grammar Combined {
                pub rule main -> (i32, i32) =
                    v1:g1::value v2:g2::value -> { (v1, v2) }
            }
        }

        Combined::parse_main
            .parse_str("a b")
            .test()
            .assert_success_is((1, 2));
    }
}
