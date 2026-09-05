// Grammar inheritance (`grammar Derived : Base`) existed up to 0.8.0 and was
// replaced by `import`. The old form must not fail deep inside generated code
// (previously: "cannot find function `num`") but name the replacement at the
// `:` itself.
use syn_grammar::grammar;

grammar! {
    grammar Base {
        pub rule num -> i32 = i:i32 -> { i }
    }
}

grammar! {
    grammar Derived : Base {
        rule main -> i32 = "add" a:num b:num -> { a + b }
    }
}

fn main() {}
