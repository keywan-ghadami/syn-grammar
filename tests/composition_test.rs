use syn::parse::Parser;
use syn_grammar::grammar;
use syn_grammar::testing::Testable;

// 1. Calc Grammar
pub mod calc_mod {
    use syn_grammar::grammar;
    grammar! {
        grammar Calc {
            pub rule num -> i32 = i:i32 -> { i }
        }
    }
}

// 2. Units Grammar
pub mod units_mod {
    use syn_grammar::grammar;
    grammar! {
        grammar Units {
            pub rule weight -> i32 =
                n:i32 "kg" -> { n * 1000000 }
              | n:i32 "g"  -> { n * 1000 }
              | n:i32 "mg" -> { n }
        }
    }
}

// 3. GrammCalc Grammar
pub mod gramm_calc_mod {
    use syn_grammar::grammar;
    // We don't need imports if we use full paths in include
    
    grammar! {
        include crate::calc_mod::Calc_rules as c;
        include crate::units_mod::Units_rules as u;

        grammar GrammCalc {
            pub rule expr -> i32 =
                l:expr "+" r:term -> { l + r }
              | t:term -> { t }

            rule term -> i32 =
                w:u::weight -> { w }
              | n:c::num -> { n }
        }
    }
}

// 4. Rechner Grammar
pub mod rechner_mod {
    use syn_grammar::grammar;
    // include crate::gramm_calc_mod::GrammCalc_rules as gc;
    
    grammar! {
        include crate::gramm_calc_mod::GrammCalc_rules as gc;
        grammar Rechner {
            pub rule main -> i32 =
                "rechne" paren(e:gc::expr) -> { e }
        }
    }
}

#[test]
fn test_composition_complex() {
    rechner_mod::Rechner::parse_main
        .parse_str("rechne ( 2 g + 13kg )")
        .test()
        .assert_success_is(13002000);
}

#[test]
fn test_composition_mixed() {
    rechner_mod::Rechner::parse_main
        .parse_str("rechne ( 500mg + 1 g )")
        .test()
        .assert_success_is(1500);
}
