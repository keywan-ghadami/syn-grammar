// The problem: we need Calc_rules and Units_rules in scope.
// But we can't import them from crate because they are generated.

use syn_grammar::grammar;

grammar! {
    include Calc_rules as c;
    include Units_rules as u;

    grammar GrammCalc {
        pub rule expr -> i32 =
            l:expr "+" r:term -> { l + r }
            | t:term -> { t }

        rule term -> i32 =
            w:u::weight -> { w }
            | n:c::num -> { n }
    }
}
