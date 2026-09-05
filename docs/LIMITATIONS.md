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

### Backtracking costs an allocation
The body of a rule runs on a `ParseBuffer`. Every backtracking point
(alternative, `?`, `*`, `+`, `peek`, `not`, `recover`, every list item) works on
a fork (`fork`) and plays it back on success (`advance_to`, O(1) according to
syn). A fork is a small `Rc` allocation; in the cursor design before,
backtracking was allocation-free.

The trade is clearly worth it: in return the `TokenBuffer` is built exactly
once instead of once per syn AST type. An argument list with 2000 `syn::Type`
entries went from 1.17 s to 5.3 ms, and quadratic became linear (compare
`any_ident` without any AST type: 3.1 ms).

Background in [`adr/adr15-linear-parsing.md`](adr/adr15-linear-parsing.md).
