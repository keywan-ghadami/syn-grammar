# Changelog

`syn-grammar-model` contains the front-end parser of the grammar DSL, the data
model and the validator. It is released in lockstep with `syn-grammar`; the
complete list is in [`syn-grammar/CHANGELOG.md`](../../syn-grammar/CHANGELOG.md).

## [0.9.0] - Draft, unreleased

> Not on crates.io yet; the last published release is 0.8.0.

### Breaking Changes

- **`parse_grammar_with_builtins` no longer exists.** The entry point for
  backend authors is `parse_grammar::<B: Backend>`.
- **`model` no longer re-exports `backend::*` and `types::*` flatly.**
  `syn_grammar_model::model::Identifier` no longer resolves; the correct path
  is `model::types::Identifier`.
- **`GrammarDefinition`** loses `inherits` and gains `extern_rules` and
  `imports`. `parser::InheritanceSpec` is gone; `grammar Foo : Base` is
  rejected by the parser with a message that names `import Base as base;`
  as the replacement.
- **`Rule`** gains `return_type_kind` and `is_lexical`; `params` is
  `Vec<RuleParameter>` instead of `Vec<(Ident, Option<Type>)>`.
- **`ModelPattern::RuleCall`** carries `rule_path: syn::Path` instead of
  `rule_name: Ident`. New variants are `LexicalScope` and `SpacedScope`
  (19 in total).
- **Uppercase rule names are automatically lexical** (`is_lexical`).

### Added

- **The "Undefined rule" message names the replacements**: `extern rule` for
  a hand-written parser, `import … as alias;` for another grammar's rule, and
  says explicitly when a `use` of the same name is what the user tried.
- **Unknown named arguments of `separated` / `repeated` are rejected**
  (`unknown argument `error` for `separated`; supported: min, trailing,
  item_label`). They used to be ignored silently.

### Fixed

- **The "Undefined rule" check** was switched off by any `use` statement; it now
  hangs on the glob import, the only thing that can bring in unknown rule
  names.
- A doc comment with `syn::parse::<Token>()` without backticks made
  `cargo doc` abort under `-D warnings`.

### Note

`docs/adr/adr1.md` (portable/backend-specific primitives) is withdrawn:
`PORTABLE_BUILTINS`/`SYN_SPECIFIC_BUILTINS` were never implemented, and
validation runs exclusively against `B::get_builtins()`.
