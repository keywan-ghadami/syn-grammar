# Limitations

## Grammar Definition

### Binding Anonymous Groups
Directly binding to an anonymous group with an action block is not supported.
`rule main = x:("a" -> { 1 })` will fail to compile.
Instead, extract the group into a named rule:
`rule main = x:my_rule`
`rule my_rule = "a" -> { 1 }`

### Left Recursion
**Direct** left recursion (`expr = expr "+" term | term`) is supported and
compiled into a loop. **Indirect** left recursion (`a -> b -> a`) is rejected at
macro time with `Indirect left recursion detected (unsupported)`.

### Undefined-Rule Checking and Glob Imports
A grammar containing a glob import (`use foo::*;`, which is also what
`grammar Foo : Base` expands to) cannot be checked for undefined rule names — the
glob may legitimately supply them. Ordinary named imports do not disable the
check.

## Diagnostics

### No Line/Column Inside a Proc Macro on Stable
`Span::start()` returns `LineColumn { line: 0, column: 0 }` for every span in a
procedural macro on stable Rust (proc-macro2, `src/wrapper.rs`). Consequences:

- The textual `at column N (line M)` suffix is omitted there — rustc underlines
  the span in the editor instead, which is the better presentation anyway.
- Error *selection* therefore does not use positions at all; it compares
  `syn::buffer::Cursor` values. See [`ERROR_HANDLING.md`](ERROR_HANDLING.md).
- Tests that go through `parse_str` take the proc-macro2 *fallback*, which does
  have positions. A message that looks fine in a test can therefore differ from
  what a macro user sees.


## Input Model

### Rust Token Trees Only
The parser consumes a `proc_macro2::TokenStream`, so the input must be
tokenizable by Rust's lexer. Languages with different lexical rules (significant
whitespace, unusual string or comment syntax) and binary formats are out of
scope. `lex(...)`/`spaced(...)` and the `whitespace` assertion recover
*adjacency* information within those limits, but they do not change the lexer.

### Rücksetzen kostet eine Allokation
Der Rumpf einer Regel läuft auf einem `ParseBuffer`. Jeder Rücksetzpunkt
(Alternative, `?`, `*`, `+`, `peek`, `not`, `recover`, jedes Listenelement)
arbeitet auf einer Gabel (`fork`) und spielt sie bei Erfolg ein (`advance_to`,
laut syn O(1)). Eine Gabel ist eine kleine `Rc`-Allokation; im Cursor-Design
davor war Zurücksetzen allokationsfrei.

Der Tausch lohnt sich deutlich: dafür wird der `TokenBuffer` genau einmal gebaut
statt einmal je syn-AST-Typ. Eine Argumentliste mit 2000 `syn::Type`-Einträgen
ging damit von 1,17 s auf 5,3 ms, und aus quadratischem wurde lineares Verhalten
(vgl. `any_ident` ohne jeden AST-Typ: 3,1 ms).

Hintergrund in [`adr/adr15-linear-parsing.md`](adr/adr15-linear-parsing.md).
