# Changelog

`syn-grammar-model` enthaelt den Frontend-Parser der Grammatik-DSL, das
Datenmodell und den Validator. Es wird versionsgleich mit `syn-grammar`
veroeffentlicht; die vollstaendige Liste steht in
[`syn-grammar/CHANGELOG.md`](../../syn-grammar/CHANGELOG.md).

## [0.10.0]

### Breaking Changes

- **`parse_grammar_with_builtins` existiert nicht mehr.** Einstieg fuer
  Backend-Autoren ist `parse_grammar::<B: Backend>`.
- **`model` re-exportiert `backend::*` und `types::*` nicht mehr flach.**
  `syn_grammar_model::model::Identifier` loest nicht mehr auf; korrekt ist
  `model::types::Identifier`.
- **`GrammarDefinition`** verliert `inherits` und gewinnt `extern_rules` und
  `imports`. `grammar Foo : Base` wird auf `use super::Base::*;` abgebildet.
- **`Rule`** gewinnt `return_type_kind` und `is_lexical`; `params` ist
  `Vec<RuleParameter>` statt `Vec<(Ident, Option<Type>)>`.
- **`ModelPattern::RuleCall`** traegt `rule_path: syn::Path` statt
  `rule_name: Ident`. Neu sind die Varianten `LexicalScope` und `SpacedScope`
  (insgesamt 19).
- **Grossgeschriebene Regelnamen sind automatisch lexikalisch** (`is_lexical`).

### Fixed

- **Die "Undefined rule"-Pruefung** wurde von jedem `use`-Statement
  abgeschaltet; sie haengt jetzt am Glob-Import, der als einziger unbekannte
  Regelnamen mitbringen kann.
- Ein Doc-Kommentar mit `syn::parse::<Token>()` ohne Backticks liess
  `cargo doc` unter `-D warnings` abbrechen.

### Note

`docs/adr/adr1.md` (Portable/Backend-spezifische Primitive) ist zurueckgezogen:
`PORTABLE_BUILTINS`/`SYN_SPECIFIC_BUILTINS` wurden nie implementiert, und die
Validierung laeuft ausschliesslich gegen `B::get_builtins()`.
