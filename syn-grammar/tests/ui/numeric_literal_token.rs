// A numeric literal cannot be used as a token in a pattern: syn tokenises `1`
// as a literal, not as punctuation or an identifier, and matching it by text
// would silently accept `1u8` or `0x1`. The grammar must say so at the
// literal and name the built-ins to use instead. (Formerly `digits.fixme`,
// a parked test that documented this limitation without pinning the message.)
use syn_grammar::grammar;

grammar! {
    grammar Digits {
        pub rule main -> () = l:letter+ d:digit+ -> { let _ = (l, d); }
        rule letter -> () = ("a" | "b" | "c") -> { () }
        rule digit -> () = ("0" | "1" | "2") -> { () }
    }
}

fn main() {}
