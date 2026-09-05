use syn_grammar::grammar;

grammar! {
    import crate::modules::gramm_calc::GrammCalc as gc;

    grammar Calculator {
        pub main -> i32 =
            "calculate" paren(e:gc::expr) -> { e }
    }
}
