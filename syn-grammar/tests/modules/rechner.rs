use syn_grammar::grammar;

grammar! {
    import crate::modules::gramm_calc::GrammCalc as gc;

    grammar Rechner {
        pub main -> i32 =
            "rechne" paren(e:gc::expr) -> { e }
    }
}
