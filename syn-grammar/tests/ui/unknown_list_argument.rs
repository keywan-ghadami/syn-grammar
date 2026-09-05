// A named argument that `separated`/`repeated` do not know used to be ignored
// silently. `error=` was even documented once; it never did anything.
use syn_grammar::grammar;

grammar! {
    grammar Lists {
        pub rule main -> Vec<i32> =
            items:separated(i32, ",", error = "need numbers") -> { items }
    }
}

fn main() {}
