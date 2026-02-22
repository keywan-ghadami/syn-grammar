use syn_grammar::grammar;

grammar! {
    grammar Units {
        pub rule weight -> i32 =
            n:i32 "kg" -> { n * 1000000 }
            | n:i32 "g"  -> { n * 1000 }
            | n:i32 "mg" -> { n }
    }
}
