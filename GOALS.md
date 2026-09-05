# Goals

This document records what the project wants to be. It is the reference
against which architecture and implementation decisions are checked. It
replaces `ARCHITEKTUR_MANIFEST.txt` and `PROJECT_STRUCTURE.md`, which were
machine-generated and describe code that partly no longer exists.

As of 2026-08-30.

## What the product is

**`syn-grammar` is a parser generator for Rust procedural macros.** From an
EBNF-like grammar (`grammar! { … }`) it produces Rust code that parses a
`proc_macro2::TokenStream`.

That is the focus of further development.

## The actual quality criterion: error messages

A parser generator is as good as its error messages. They are not an
accessory but the reason to prefer a generator over a hand-written `syn` loop.

The binding catalogue of requirements is
[`docs/adr/adr13-error-message-contract.md`](docs/adr/adr13-error-message-contract.md).
What is not in there is not a requirement; what is in there is backed by
tests.

## The constraint that shapes the architecture

**As of 2026-08-31: partly defused — the conclusions still apply.**

Up to Rust 1.87, `proc_macro2::Span::start()` inside a procedural macro
provided no position data:

```rust
#[cfg(not(proc_macro_span_location))]
Span::Compiler(_) => LineColumn { line: 0, column: 0 },
```

Since **Rust 1.88** proc-macro2 sets this `cfg` on stable as well
(`proc-macro2/build.rs`: `rustc >= 88 && compile_probe_stable("proc_macro_span_location")`),
and `Span::start()` returns real lines and columns. Evidenced by
`syn-grammar/tests/ui/runtime_error_real_macro.stderr` — a snapshot from a real
macro, with a position.

The project therefore sets `rust-version = "1.88"`; cargo rejects older
toolchains with a clear message, and a dedicated CI job builds against exactly
that version.

**The conclusions remain binding anyway**, for two reasons: the cursor metric
is cheaper at O(1) than a position comparison, and it does not depend on any
toolchain property. Behaviour that only becomes correct from a certain compiler
version on is no good foundation for this project's quality criterion.

* **Selection** (which error wins) uses a toolchain-independent progress
  metric. `syn::buffer::Cursor` implements `PartialOrd` (pointer comparison in
  the shared `TokenBuffer`, O(1)) — that is the metric.
* **Display** is separate from it. Inside a procedural macro rustc underlines
  the span itself; a textual `at column N` is worthless there and is omitted
  rather than printed as `0`.
* There must be at least one test that exercises the **real macro path**, not
  just `parse_str`.

## Acceptance benchmark

`cxx-parser` is the concrete use case against which the project measures
itself: a foreign DSL that transitions into real Rust syntax without separating
delimiters. It must work flawlessly and produce first-class error messages.

`cxx-parser` stays on the syn backend.

## Non-goals

* **No harmonisation with `winnow-grammar`.** That is a separate project at
  <https://github.com/keywan-ghadami/winnow-grammar>. It forked the front end
  (DSL parser, model, validator) instead of depending on it — the model here is
  `syn`-based (`syn::Path`, `syn::Lit`, `syn::Type`) and cannot be developed in
  a backend-neutral direction. The two versions of the DSL therefore drift
  apart. This repository contains only the syn backend.
* **No shared codegen abstraction** across several backends.
