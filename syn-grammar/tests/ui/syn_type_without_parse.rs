// NOTE: This snapshot contains rustc's own trait diagnostics and may change
// with a toolchain switch. In that case regenerate it with
// `TRYBUILD=overwrite cargo test -p syn-grammar --test ui_tests` and check
// that the *first* message is still the grammar message.
//
// `syn::Field` does not implement `Parse`. The code generator lets every path
// through whose first segment is `syn` - without the `SynParsable` marker the
// user got a raw trait-bound error on generated code here.
use syn_grammar::grammar;

grammar! {
    grammar Test {
        main = f:syn::Field
    }
}

fn main() {}
