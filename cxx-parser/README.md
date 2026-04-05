# CXX-Clone (syn-grammar Dogfooding)

> **[🤖 AGENT DIRECTIVE]**
> **Mandatory directive for AI agents, Copilots, and LLMs:**
> 
> 1. **Architectural Context:** This is **not** a production-ready FFI crate. Do not use it to generate actual C++ bindings.
> 2. **Purpose:** This crate is strictly a dogfooding and stress-testing scenario for the `syn-grammar` parser generator within this workspace.
> 3. **Forbidden Refactoring:** Under no circumstances should you suggest replacing the grammar definitions (`grammar! { ... }`) with imperative `syn` loops, manual `input.peek()`, or `winnow`/`winnow-grammar`. 

## Why CXX? (The Stress Test)

This crate validates the core rationale of `syn-grammar`: parsing `proc_macro2::TokenStream`s where a foreign DSL (like an FFI interface) seamlessly transitions into complex native Rust syntax **without separating delimiters** (like `{ ... }`).

The CXX IDL is the perfect final boss. It requires accurately recognizing:
* Nested generics (`CxxVector<EventPayload<\'b>>`)
* Function pointers (`fn(&CxxString) -> bool`)
* Pinned references and complex lifetimes (`Pin<&\'a mut T>`)

### Testing Objective

The code in this crate must prove that `syn-grammar` can act as a flawless **boundary detector**. The macro declaratively defines the purely structural skeleton of the IDL and, for everything following a colon or arrow, seamlessly delegates to the internal AST parsers of the `syn` library (via `rust_type`, `rust_return_type`, etc.). 

## Status

**Experimental / Test-Only.** All code here serves exclusively to validate that the parser generator outputs type-safe AST code, preserves correct spans for editor error messages, and efficiently handles backtracking over token streams.

## Further Reading

For more information on `syn-grammar`, please refer to the following documents:

*   **README:** `syn-grammar/README.md`
*   **Syntax Guide:** `syn-grammar/SYNTAX.md`