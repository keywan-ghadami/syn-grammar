// HINWEIS: Dieser Schnappschuss enthaelt rustc-eigene Trait-Diagnostik und kann
// sich mit einem Toolchain-Wechsel aendern. Dann mit
// `TRYBUILD=overwrite cargo test -p syn-grammar --test ui_tests` neu erzeugen und
// pruefen, dass die *erste* Meldung weiterhin die Grammatik-Meldung ist.
//
// `syn::Field` implementiert kein `Parse`. Der Codegenerator laesst jeden Pfad
// durch, dessen erstes Segment `syn` heisst - ohne den Marker `SynParsable`
// bekam der Nutzer hier einen rohen Trait-Bound-Fehler auf generierten Code.
use syn_grammar::grammar;

grammar! {
    grammar Test {
        main = f:syn::Field
    }
}

fn main() {}
