// Ein gewoehnlicher (nicht-Glob) `use` darf die "Undefined rule"-Pruefung nicht
// abschalten. Vorher galt `should_validate_rule_calls = grammar.uses.is_empty()`,
// wodurch dieser Tippfehler unbemerkt durchging und erst als Folgefehler im
// generierten Code auftauchte.
use syn_grammar::grammar;

pub struct Stmt;

grammar! {
    grammar Test {
        use super::Stmt;

        main = tyop_fehler
    }
}

fn main() {}
