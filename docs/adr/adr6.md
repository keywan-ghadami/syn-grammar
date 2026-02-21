# ADR 006: Namespaced Grammar Composition via Push-Down Accumulation

**Date:** 2026-02-21
**Status:** Done

## Context
`syn-grammar` and downstream backends (such as `winnow-grammar`) require a mechanism to modularize grammars across multiple files. Previous approaches (like runtime traits or loading external files via `include_str!`) failed because they either bypassed the 100% static validation (left-recursion, shadowing checks) or destroyed token spans, resulting in useless IDE error messages. Furthermore, end-users need a clean, flat syntax without deeply nested macro calls ("macro matryoshka") and require safe namespaces to prevent naming collisions.

## Decision
We have implemented a **Namespaced Grammar Mixin Pattern** based on macro push-down accumulation. 
This shifts the module composition entirely into the pre-AST phase of the Rust compiler (token expansion), long before `syn-grammar-model` begins its validation. 

The system consists of three architectural pillars:

### 1. Auto-Generation of Reusable Macros
For every defined `grammar!` (e.g., `grammar Base { ... }`), the backend not only generates the final Rust module but *additionally* emits a declarative macro (`macro_rules! Base_rules`). This macro serves as the carrier for the raw token stream of that grammar.

### 2. Syntactic Sugar & Push-Down Accumulation
The frontend macro (`grammar!`) accepts a new, flat include syntax with an explicit alias:
```rust
grammar! {
    include Whitespace_rules as w;
    include Math_rules as m;
    
    grammar MyLanguage { ... }
}
```
The frontend parses these `include` statements and transcribes them into a nested macro call chain. Each invoked macro (e.g., `Whitespace_rules!`, `Math_rules!`) injects its own tokens into a continuously growing array and passes the alias (`as w`, `as m`) inward, until the entire accumulated token stream is handed over to the final `grammar_core!` backend macro.

### 3. Namespace Resolution & Mangling
To prevent name collisions and establish clear boundaries, referenced EBNF rules from included grammars are invoked using their alias (e.g., `w::ws`).

* **Model (`syn-grammar-model`):** The AST (specifically the `RuleCall` node) has been expanded to store paths instead of simple identifiers (`RuleReference { namespace: Option<Ident>, name: Ident }`).
* **Codegen:** When compiling into flat Rust functions, deterministic name mangling using a double underscore is applied: the call `w::ws` is translated into the function signature `parse_w__ws(input)`.

## Consequences

### Positive
* **Perfect Spans:** Since raw token streams (including their original metadata) are passed through, IDE error messages for syntax or shadowing errors point exactly to the correct line in the referenced source file.
* **100% Static Safety:** After expansion, `syn-grammar-model` receives a single, unified AST. This allows the analysis of left-recursion and shadowed alternatives to function flawlessly across all included files.
* **Zero-Cost Abstraction:** Because no runtime traits are utilized, the performance of the generated code remains maximal (this is especially crucial for downstream high-performance backends like `winnow-grammar`).
* **Hygiene:** The double-underscore mangling reliably prevents any collisions with locally defined user rules.

### Negative / Risks
* **Macro Recursion Limit:** Including an extremely high number of modules (> 64) could theoretically hit Rust's standard macro recursion limit. However, this is an unrealistic scenario for typical grammars.
* **Frontend Complexity:** The initial parsing step of the `grammar!` macro must be robust enough to cleanly transform the token streams into the push-down chain before handing them off to the model parser.
