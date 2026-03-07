use syn_grammar::grammar;

grammar! {
    import crate::modules::calc::Calc as c;
    import crate::modules::units::Units as u;

    grammar GrammCalc {
        pub expr -> i32 =
            l:expr "+" r:term -> { l + r }
          | t:term -> { t }

        term -> i32 =
                w:u::weight -> { w }
              | n:c::num -> { n }
    }
}
