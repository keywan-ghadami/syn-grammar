use syn_grammar::grammar;

grammar! {
    grammar Test {
        rule main -> () = ")" -> { () }
    }
}

fn main() {}
