use syn_grammar::grammar;

grammar! {
    grammar Test {
        rule main -> () = ~ "a" -> { () }
    }
}

fn main() {}
