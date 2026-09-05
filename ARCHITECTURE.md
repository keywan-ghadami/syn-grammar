# Architecture (as is)

Describes how the code actually looks on 2026-08-30 on `logic-changes` — not
how it was meant to be. Goals are in [`GOALS.md`](GOALS.md).

All line numbers are checked against the state of this document.

## Structure

The path from grammar to parser has three stages:

```
grammar! { … }                          macro input (TokenStream)
      │
      ▼
core/syn-grammar-model                  FRONT END, used backend-independently
      parser.rs      1262 lines  TokenStream → syntactic AST
      model.rs        364 lines  → semantic model (ModelPattern, 19 variants, :61)
      validator.rs    527 lines  ambiguity, shadowing, indirect left recursion
      analysis.rs    1153 lines  bindings, nullable, cut, left recursion, token resolution
      │                          entry: parse_grammar::<B: Backend> (syn_grammar_model.rs:30)
      ▼
syn-grammar/syn-grammar-macros          CODEGEN (syn backend)
      codegen/rule.rs      273 lines  rules, variants, left recursion
      codegen/pattern.rs   687 lines  the 19 patterns
      monomorphize.rs      420 lines  resolve generics at macro time
      backend.rs           262 lines  built-in catalogue of the syn backend
      │
      ▼
core/grammar-kit + syn-grammar/src      RUNTIME, called by the generated code
```

The generated code addresses the runtime through the alias `rt`
(`syn-grammar/src/syn_grammar.rs:5-9`), which bundles `grammar_kit::*`,
`builtins` and `token_filter`.

## Front end: `core/syn-grammar-model`

Parses the DSL, transforms it into the semantic model and validates it. Entry
`parse_grammar::<B: Backend>(TokenStream)` (`syn_grammar_model.rs:30-41`):
`syn::parse2` → `.into()` → `validator::validate::<B>`.

`ModelPattern` (`model.rs:61`) has **19** variants: `Cut`, `Lit`, `RuleCall`,
`Group`, `Bracketed`, `Braced`, `Parenthesized`, `Optional`, `Repeat`, `Plus`,
`SpanBinding`, `Recover`, `Peek`, `Not`, `Until`, `Count`, `LexicalScope`,
`SpacedScope`, `Fail`.

The `Backend` trait (`model/backend.rs:13-16`) has exactly one method,
`get_builtins() -> &'static [BuiltIn]`. It drives **only the validation** of
built-in names — it says nothing about codegen. There is no shared codegen
abstraction.

**On the name:** the crate was called `syn-grammar-model` because it was once
used by two backends. Since `winnow-grammar` moved out (2026-08-31) it has
only one user, and the name fits again. It was backend-independent only in its
*use*, not in its *types*: the model carries `syn::Path`, `syn::Lit`,
`syn::Type`, `syn::ItemUse` (`model.rs:14,20-23,28,65,69-70`), and
`analysis::resolve_token_types` / `analysis::get_simple_peek` produce
`Token![…]` and `syn::token::*` types. That is exactly why winnow forked the
crate when moving out instead of depending on it — only that way can it be
developed towards `syn`-independence there.

## Runtime: `core/grammar-kit`

The body of a rule works on a **stream** (`ParseBuffer`), the leaf primitives
still on the **cursor**:

```rust
pub type Strom<'a>            = syn::parse::ParseBuffer<'a>;          // stream.rs
pub type StreamResult<'a, T>  = Result<T, ParseError<'a>>;            // stream.rs
pub type ParseResult<'a, T>   = Result<(T, Cursor<'a>), ParseError<'a>>; // error.rs
```

A rule is `fn parse_x_impl<'a>(input: &Strom<'a>, ctx: &mut ParseContext<'a>)
-> StreamResult<'a, T>`. Deliberately `&Strom<'a>` and **not** syn's alias
`ParseStream<'a> = &'a ParseBuffer<'a>`: the alias equates the lifetime of the
reference with that of the tokens, so an `input.fork()` would shorten `'a` to
the stack frame — errors from a fork could then no longer leave the call. And
those are exactly what error selection needs.

Backtracking goes through `gabel` (`fork`) and `uebernehmen` (`advance_to`,
O(1) according to syn): an attempt runs on the fork, only success is played
back. Errors are return values and are combined via `ParseError::merge`
(`error.rs`) — there is **no** global error state.

| File | Contents |
|---|---|
| `error.rs` | `ParseError` (span, `at` cursor, message, priority, `is_fatal`, rule_stack), `merge`, `Display` |
| `context.rs` | `ParseContext`: scopes, lexical mode, `last_span`, `furthest` — **without** error state |
| `stream.rs` | `Strom`, `parse_syn`, `parse_mit`, `gabel`/`uebernehmen`, `gruppe`, `schritt`, `token_nehmen` |
| `combinators.rs` | `peek_syn`, `take_single`/`SingleToken`, `parse_separated`, `parse_repeated`, `finish_variants` |
| `testing.rs` | `Testable`/`TestResult`, `assert_failure_contains` (substring comparison) |

`parse_syn` (`stream.rs`) is the access to syn's `Parse` impls and simply an
`input.parse::<T>()` — O(length of the type). Until August 2026 this went
through a bridge that materialised the remaining stream and let
`Parser::parse2` build a new `TokenBuffer` from it; that was O(rest) per call
and hence quadratic in the length of a list. See
`docs/adr/adr15-linear-parsing.md`.

Conversely, single tokens stay on the cursor: `schritt` runs a cursor primitive
inside a `ParseBuffer::step` episode and advances the stream by exactly its
result. `step` demands a closure that works for **every** lifetime, which is why
a `ParseError<'c>` cannot leave it; `schritt` carries the error through without
its cursor and re-attaches it outside at the entry position. That is not an
approximation — these primitives report their error there anyway.

Into delimiter groups, `gruppe` descends via
`syn::__private::parse_{parens,braces,brackets}`. `AnyDelimiter::parse_any_delimiter`
does not work: its return value is shortened to the lifetime of `&self`, so no
error from inside the group would carry outward.

## Known weaknesses

Evidenced, not assumed:

1. ~~**Diagnostics do not work in production use.**~~ *Done.* `merge` compares
   `Cursor` via `PartialOrd` (O(1), `src/buffer.rs`) instead of `span.start()`.
   The original reason — `(0,0)` inside a procedural macro — no longer applies
   since Rust 1.88 anyway; the project requires that version. The cursor metric
   stays because it is cheaper and toolchain-independent. See `GOALS.md`.

2. ~~**The bridge call for syn AST types remains O(n).**~~ *Done* (ADR 15,
   stage 3). The body runs on a `ParseBuffer` that is built exactly once; a
   `syn::Type` costs `input.parse::<T>()`. Measured on a generated argument list
   with 2000 entries: 1.17 s → 5.3 ms, and quadratic became linear. The price is
   an `Rc` allocation per backtracking point instead of a cursor copy.

3. **Missing diagnostics building blocks.** `expected one of: …`, label
   bubbling, item index in the rule stack (`in item 3`) existed before the
   rebuild and have been missing since.

4. ~~**Dead code.**~~ *Done.* `transaction.rs` (147 lines, never declared as a
   module), `macros.rs` (`test_both_backends!`, gated on non-existent features)
   and the empty features `rt`/`trace` are removed. `test_both_backends!` was
   moreover unfixable in principle: its body needs `syn-grammar` and
   `winnow-grammar`, which both depend on `grammar-kit` — a cycle. Its doctest
   only counted as green because the macro expanded to nothing.

## `cxx-parser`

Acceptance benchmark on the syn backend (`cxx-parser/Cargo.toml:8`). 5 rules,
`src/cxx_parser.rs:37-79`. The interesting part is the hand-over to syn for
everything after `:` and `->` (`syn::Type`, `syn::ReturnType`, `syn::Generics`,
`syn::Macro`) — exactly the boundary where a foreign DSL transitions into real
Rust syntax.

## `winnow-grammar` — moved out

Was a second backend on the same front end until 2026-08-31. It now lives at
<https://github.com/keywan-ghadami/winnow-grammar> and is fully independent: no
reference left to `syn-grammar`, `syn-grammar-model` or `grammar-kit`.

Resolved when moving out: the front end was forked as `winnow-grammar-model`;
from `grammar-kit` only `WithSpan` (4 lines) and `testing.rs` (341 lines, no
`syn` involvement) moved along, the crate itself was not forked. The actual
blocker was not in the manifests but in the generated code:
`codegen/variants.rs` wrote `::grammar_kit::WithSpan` as an absolute crate path,
so every user crate needed `grammar-kit` directly.

Four dependencies were dead (not a single import): `syn` and
`syn-grammar-model` in `winnow-grammar`, `syn-grammar` and `grammar-kit` in
`winnow-grammar-macros` — where `syn-grammar` pulled the complete syn backend
into every winnow build.

`docs/adr/adr14-shared-context-pattern.md` moved along.

## Outdated documents

* `ARCHITEKTUR_MANIFEST.txt` — describes `core/grammar-kit/src/lib.rs`; the file
  has not existed since the rebuild.
* `PROJECT_STRUCTURE.md` — speculatively worded ("likely contains"), names
  neither `grammar-kit` nor `syn-grammar-model`, refers to a non-existent
  `testresults.txt`.
* `EXTENDING.md` — describes an API that does not exist
  (`parse_grammar_with_builtins`, `Lit(LitStr)`, 6 instead of 19 patterns); the
  example code would not compile.
