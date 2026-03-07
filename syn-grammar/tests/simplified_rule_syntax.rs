use syn::parse::Parser;
use syn_grammar::grammar;

mod simplified {
    use super::*;
    grammar! {
        grammar Simplified {
            // Original syntax
            pub full -> () = "full" -> { () }

            // Simplified: no rule keyword
            pub no_kw -> () = "nokw" -> { () }

            // Simplified: no rule keyword, no return type (defaults to ())
            pub no_ret = "noret" -> { () }

            // Simplified: no rule keyword, no return type, no action (defaults to {()})
            pub short = "short"

            // Simplified with pub
            pub myrule = "a"
        }
    }
}

#[test]
fn test_simplified_syntax() {
    simplified::Simplified::parse_myrule.parse_str("a").unwrap();
    simplified::Simplified::parse_short
        .parse_str("short")
        .unwrap();
    simplified::Simplified::parse_no_ret
        .parse_str("noret")
        .unwrap();
    simplified::Simplified::parse_no_kw
        .parse_str("nokw")
        .unwrap();
    simplified::Simplified::parse_full
        .parse_str("full")
        .unwrap();
}
