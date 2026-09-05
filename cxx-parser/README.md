# cxx-parser

A parser for the [`#[cxx::bridge]`](https://cxx.rs) interface definition
language, written entirely in `syn-grammar`.

**Test-only, and not published.** It generates no C++ and produces no bindings;
it exists to answer one question about the parser generator in this workspace:
does it hold up on a real language, or only on examples? The grammar
definitions are the thing under test — replacing them with hand-written `syn`
loops, `input.peek()` or a different parser library would delete the test.

## Why CXX

The bridge IDL is the hardest case this workspace has: **a foreign DSL that
turns into Rust without a delimiter to mark the transition.** After a `:` or a
`->`, the input is simply a Rust type, and the parser has to hand over to `syn`
mid-rule and take back over at exactly the right token:

```rust
fn dispatch<'a, 'b>(
    self: Pin<&mut BlobstoreClient>,
    payload: &'a mut CxxVector<Payload<'b>>,
    filter: fn(&CxxString, Option<&[u8]>) -> bool,   // commas inside a comma-separated list
) -> Result<UniquePtr<BlobMetadata>>;
```

On top of that the IDL is genuinely ambiguous at its start: five kinds of module
item and three kinds of extern item, and every one of them may begin with
attributes — so not a single alternative can be decided by its first token.

## What the grammar covers

The bridge language as the cxx book documents it: `use` statements, shared
structs, shared enums with optional (and negative) discriminants,
`extern "C++"` and `extern "Rust"` blocks, `include!`, opaque types, type
aliases, free functions and methods with all four receiver forms (`&self`,
`&mut self`, `self: &T`, `self: Pin<&mut T>`), `unsafe fn`, generic lifetime
parameters, attributes (`#[namespace]`, `#[rust_name]`, doc comments), and the
`impl UniquePtr<T> {}` instantiations.

Not covered: everything past parsing. Name resolution, the C++ side, and the
semantic rules cxx enforces on top of the syntax (a receiver only as the first
parameter, `Result` only in return position) are out of scope.

## What it exercises

| Feature | Where |
|---|---|
| Hand-over to `syn` mid-rule | `syn::Type`, `syn::Path`, `syn::ReturnType`, `syn::Generics`, `syn::ItemUse` |
| Alternatives behind a nullable prefix | every item rule starts with `outer_attrs` |
| Alternative labels (`#`) | `mod_item`, `extern_item` — they are what makes `expected one of:` readable |
| Cut (`=>`) | after `type`, `fn`, `impl`, `extern`: commits so the error stays inside the right item |
| Lists with labels | `separated(…, item_label="function parameter")` and the struct/enum lists |
| `extern rule` | `extern_lang` checks the *content* of the language string, which the grammar cannot |
| `fail(…)` | states why an `impl` body must be empty |
| Delimiters and `eof` | `paren(…)`, `{ … }`, the empty `impl` body |

## Running it

```sh
cargo test -p cxx-parser                       # 31 tests: 14 on the AST, 17 on the messages
cargo run  -p cxx-parser -- some/bridge.rs     # summary, or the error
echo 'mod ffi { extern "Java" { } }' | cargo run -q -p cxx-parser
```

The last one prints what this crate is really about:

```text
error: expected "C++" or "Rust", found "Java" at column 17 (line 1)
in extern block
in mod item
in top level mod
```

`tests/error_messages.rs` checks those messages point by point against
[`docs/adr/adr13-error-message-contract.md`](../docs/adr/adr13-error-message-contract.md),
which is the binding contract for diagnostics in this project.

## Further reading

* [`syn-grammar/README.md`](../syn-grammar/README.md) — the crate under test.
* [`syn-grammar/SYNTAX.md`](../syn-grammar/SYNTAX.md) — the grammar language.
* [`GOALS.md`](../GOALS.md) — why this crate is the acceptance benchmark.
