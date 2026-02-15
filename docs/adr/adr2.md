# Architecture Decision Record (ADR) 2: Explicit Backend Contracts and Portable Types

## 1. Title

Explicit Backend Contracts and Portable Types for `syn-grammar`

## 2. Status

Proposed (Supersedes implementation details of ADR 1). Implementation in progress (Iteration 1).

## 3. Context

ADR 1 established the distinction between `PORTABLE_BUILTINS` and `SYN_SPECIFIC_BUILTINS`. However, it left the implementation details of how to enforce this contract and handle return types vague.

A critical issue identified is "Action Block Type Leakage". If `ident` returns `syn::Ident` in the default backend but `&str` in a `winnow` backend, a grammar using `ident` is not truly portable because the user's action code (e.g., `{ name.to_string() }`) might depend on the specific API of the return type.

To achieve true portability, the grammar must guarantee that portable primitives return **portable types** (or standard Rust types) regardless of the backend.

## 4. Decision

We are implementing a 3-iteration plan to formalize the backend contract and introduce portable types.

### Iteration 1: Formalize Backend Contract and Portable Types (Current Status: In Progress)

**Goal:** Replace implicit "magic string" built-ins with a formal, type-checked contract and introduce portable wrapper types for key primitives.

**Changes:**
1.  **`Backend` Trait:** Introduce `syn_grammar_model::Backend` trait.
    ```rust
    pub trait Backend {
        fn get_builtins() -> &'static [BuiltIn];
    }
    pub struct BuiltIn { name: &'static str, return_type: &'static str }
    ```
2.  **Portable Types:** Introduce `syn_grammar_model::model::types::Identifier` and `StringLiteral`.
    *   `Identifier`: Wraps `String` and `Span`.
    *   `StringLiteral`: Wraps `String` and `Span`.
3.  **`SynBackend` Implementation:**
    *   The default backend (`SynBackend`) now declares `ident` as returning `Identifier` and `string` as returning `StringLiteral`.
    *   **Breaking Change:** Users of `syn-grammar` will now receive `Identifier` instead of `syn::Ident` in their action blocks for the `ident` rule.
4.  **`CommonBuiltins` Trait:**
    *   Refactor the runtime (`src/builtins.rs`) to use a `CommonBuiltins` trait.
    *   This allows the parsing logic for portable primitives to be abstract over the underlying stream, provided it implements the trait.

### Iteration 2: Introduce a Common `Spanned<T>` Wrapper (Planned)

**Goal:** Provide a portable way to access source location data (Spans) for any return type.

**Plan:**
1.  Define `pub struct Spanned<T> { value: T, span: Span }` in `syn-grammar-model`.
2.  Update backends to support "spanned" variants of rules (or fully valid spanned return types) using this wrapper.
3.  Ensure `Span` is abstract enough (or standardized on `proc_macro2::Span` where appropriate) to be useful across backends.

### Iteration 3: Introduce Agnostic Core Data Types via New Built-ins (Refined)

**Goal:** complete the set of portable types.

**Plan:**
1.  Expand `syn_grammar_model::model::types` to cover other complex primitives if necessary.
2.  Ensure that all `PORTABLE_BUILTINS` have a defined, backend-agnostic return type (e.g., `char`, `u32`, `Identifier`, `StringLiteral`).

## 5. Consequences

*   **Breaking Changes:** The default `syn` backend now returns `Identifier` for `ident`. Existing grammars expecting `syn::Ident` will fail to compile and must be updated. This is acceptable for the `0.x` release cycle to achieve clean architecture.
*   **Type Safety:** The validation step now checks if the user's grammar expects a type that matches the backend's declared return type for a built-in.
*   **True Portability:** A user writing `name: ident` can now write action code against `Identifier` (e.g., `name.text`) that will work identically on `syn` and `winnow` backends.

## 6. Current Status (Refactoring)

*   **Completed:**
    *   Defined `Identifier`, `StringLiteral`, `Backend` trait, `BuiltIn` struct in `syn-grammar-model`.
    *   Implemented `SynBackend` in `syn-grammar-macros` with the new types.
    *   Refactored `syn-grammar/src/builtins.rs` to use `CommonBuiltins` and return new types.
    *   Updated `codegen` to pass `&mut input` to `_impl` functions.
*   **Pending:**
    *   Fixing integration tests (`tests/*.rs`) that are failing due to type mismatches (`syn::Ident` vs `Identifier`, `String` vs `StringLiteral`).
