# ADR 008: Black-Box Grammar Composition via Extern and Import Interfaces

**Date:** 2026-02-22
**Status:** Accepted (Supersedes ADR 006)

## Context
In ADR 006, we attempted to solve cross-file grammar composition using a "White-Box" approach (Push-Down Accumulation via `macro_rules!`). The goal was to maintain a single, global Abstract Syntax Tree (AST) to preserve 100% static validation (left-recursion and shadowing detection) across module boundaries.

However, practical implementation revealed severe friction with Rust's macro hygiene and module visibility rules:
1. **Visibility Leaks:** Deeply nested macro invocations forced internal dependency paths to be exposed to outer consuming modules.
2. **Path Resolution Failures:** Re-exporting macros and invoking them via paths (e.g., `crate::module::Macro!`) led to unresolvable `no rules expected ::` errors due to compiler limitations on recursive macro expansions.
3. **Developer Experience:** Debugging the resulting "macro-matryoshka" became unsustainable for both the maintainers and the end-users.

We need a composition mechanism that respects standard Rust module semantics, compiles quickly, and provides a stable API, even if it means sacrificing global static analysis across grammar boundaries.

## Decision
We are abandoning the White-Box AST merging strategy. Instead, we are adopting a **Black-Box Interface Strategy**. 

Grammars will now treat external rules and other grammars as compiled, opaque Rust functions. We are introducing two new syntax constructs to the `grammar!` DSL:

### 1. `extern rule` (Function Binding)
Allows binding a grammar rule directly to an existing Rust function in the current scope. The macro generates no parsing logic for this rule, but trusts the user-provided signature.
```rust
grammar! {
    grammar MyGrammar {
        // Declares an external function `parse_custom_string(input) -> String`
        extern rule custom_string -> String;

        pub rule main -> String = 
            "prefix" s:custom_string -> { s }
    }
}
```

### 2. `import grammar` (Module Binding)
Allows importing an entire generated grammar module. Rules from the imported grammar can be accessed via an alias.

```rust
grammar! {
    // Tells the model that `math::...` refers to `crate::math_parser::MathGrammar`
    import crate::math_parser::MathGrammar as math;

    grammar MyLanguage {
        // Calls `crate::math_parser::MathGrammar::parse_expr(input)` at runtime
        pub rule statement -> Stmt = 
            "let" ident "=" math::expr ";" -> { Stmt::Let(ident, math) }
    }
}
```

## Architectural Changes

### `syn-grammar-model`
* **AST Additions:** Add `ExternRule` and `ImportedGrammar` nodes to the model definition.
* **Validation Boundary:** The validator will treat `extern` and `import` calls as terminal leaf nodes. It will *not* attempt to resolve them or check them for left-recursion or shadowing. The static guarantees are strictly local to the current `grammar!` block.

### `syn-grammar-macros` (Codegen)
* **No Macro Generation:** We no longer generate `macro_rules!` on the fly. We strictly generate the `pub mod ParserName` with its parsing functions.
* **Call Translation:** When the codegen encounters `math::expr`, it directly translates this into the Rust function call `<ImportedPath>::parse_expr(input)`.

## Consequences

### Positive
* **Radical Simplification:** Eliminates all complex macro-recursion and push-down accumulation logic. The codebase becomes drastically easier to maintain.
* **Native Rust Semantics:** Users can utilize standard Rust visibility (`pub`, `pub(crate)`) and paths. The system behaves exactly as a Rust developer intuitively expects.
* **Compile-Time Performance:** Grammars are compiled in isolation. We no longer duplicate AST tokens across the crate, resulting in significantly faster build times.

### Negative
* **Loss of Global Validation:** We lose the ability to detect left-recursion loops or shadowing that cross grammar boundaries. If Grammar A calls Grammar B, and B calls A, it will result in a runtime stack overflow rather than a compile-time error.
* **No Rule Overriding:** Because composition is now functional (calling out to a black box) rather than structural (merging ASTs), users cannot inherit a grammar and "override" a specific rule within it.
