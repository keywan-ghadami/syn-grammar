use syn::parse::Parser;
use syn_grammar::grammar;

pub mod single_mod {
    use syn_grammar::grammar;
    grammar! {
        grammar SingleGrammar {
            pub rule single_rule -> u32 = "s" -> { 99 }
        }
    }
}

#[test]
fn test_single_grammar() {
    single_mod::SingleGrammar::parse_single_rule
        .parse_str("s")
        .unwrap();
}
