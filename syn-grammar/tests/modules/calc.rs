use syn_grammar::grammar;

grammar! {
    grammar Calc {
        pub rule num -> i32 = i:i32 -> { i }
    }
}
