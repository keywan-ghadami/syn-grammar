// An ordinary (non-glob) `use` must not switch off the "Undefined rule" check.
// Previously `should_validate_rule_calls = grammar.uses.is_empty()` applied,
// so this typo passed unnoticed and only showed up as a follow-up error in the
// generated code.
use syn_grammar::grammar;

pub struct Stmt;

grammar! {
    grammar Test {
        use super::Stmt;

        main = tyop_error
    }
}

fn main() {}
