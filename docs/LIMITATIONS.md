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

### Item Error vs. Separator Error at the Same Position
When a list item and the following separator both fail at the same token, the
separator error currently wins, producing ``expected `,` `` where
`expected <item>` would be more useful. Tracked in
`cxx-parser/tests/error_messages.rs::ungueltiges_argument_wird_noch_zu_schwach_gemeldet`.

## Input Model

### Rust Token Trees Only
The parser consumes a `proc_macro2::TokenStream`, so the input must be
tokenizable by Rust's lexer. Languages with different lexical rules (significant
whitespace, unusual string or comment syntax) and binary formats are out of
scope. `lex(...)`/`spaced(...)` and the `whitespace` assertion recover
*adjacency* information within those limits, but they do not change the lexer.

### Quadratisch bei vielen syn-AST-Typen in einer Liste
Ein `syn::Type` (und Verwandte) wird über eine Brücke geparst, die den Reststrom
bis zum Ende der umschließenden Delimiter-Gruppe materialisiert. Bei einem
solchen Typ je Listenelement wächst die Zeit quadratisch in der Länge dieser
Liste: gemessen 3,4 ms bei 100 Elementen, 1,4 s bei 2000. Alles andere ist
linear. Ursache und Auswege in [`adr/adr15-linear-parsing.md`](adr/adr15-linear-parsing.md).
