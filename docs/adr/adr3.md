# Architecture Decision Record (ADR) 003: Higher-Order Generic Rules, Macro-Time Monomorphization, and Trait Bound Preservation

**Date:** 2026-02-15
**Status:** Accepted
**Context:** The `syn-grammar-model` serves as the unified frontend for defining declarative EBNF-like parsers, supporting both token-based (`syn-grammar`) and text-based (`winnow-grammar`) backends. To adhere to DRY (Don't Repeat Yourself) principles, the DSL requires a robust mechanism for reusable higher-order parsing patterns (e.g., comma-separated lists, bracketed sequences). The architectural challenge is implementing generic rule arguments and strict trait bounds without requiring a custom, fragile type parser, and without obscuring the static call graph required for automatic left-recursion detection.

---

## 1. Architectural Decision

The toolchain will implement **Higher-Order Generic Rules** using a combination of standard Rust syntax for declaration and **Macro-Time Monomorphization** for code generation. 

Instead of relying on the Rust compiler (`rustc`) to resolve abstract generic function bounds (`impl Parser<T>`) at type-check time, the `syn-grammar-model` procedural macro will act as a template engine. It will instantiate, type-check (via static assertions), and flatten generic rules into concrete functions during the macro expansion phase.



### 1.1. Syntax and Frontend Strategy
To avoid maintaining a custom type parser, the DSL strictly utilizes standard Rust tokens natively supported by `syn::Type` and `syn::Generics`.

* **Declaration:** `rule key_value_map<K: Hash + Eq, V>(k_parser, v_parser) -> HashMap<K, V> = ...`
* **Generic Parameters (`<K: Hash + Eq, V>`):** Parsed natively as `syn::Generics`. This captures both the type variables and their associated trait bounds.
* **Rule Arguments (`k_parser`, v_parser`):** Left untyped in the DSL signature. The macro implicitly treats these arguments as substitutable AST nodes (sub-rules or inline patterns).
* **Return Type inference (`-> [T]`):** For simple lists, the DSL supports `[T]` (parsed as `syn::Type::Slice`), which acts as syntactic sugar for `Vec<T>`.

### 1.2. The Resolution Engine (Macro-Time Monomorphization)
When a generic rule is invoked (e.g., `key_value_map(string_ident, integer)`), the frontend performs a deterministic AST transformation before passing the syntax tree to the backends:

1. **Registry Lookup:** Identifies the target arguments (`string_ident`, `integer`) and extracts their previously registered return types (e.g., `String`, `i32`).
2. **Type Inference & Binding:** Positively matches the generic parameters to the resolved types: `K = String`, `V = i32`.
3. **Template Cloning:** Deep-copies the entire AST of the `key_value_map` rule.
4. **AST Substitution:** A `syn::visit_mut::VisitMut` walker traverses the cloned AST (including the action block and return type) and replaces all instances of `K` and `V` with `String` and `i32`, respectively. It also replaces the untyped parameter invocations (`k_parser`, `v_parser`) with direct calls to the concrete rules.

### 1.3. Trait Bound Preservation (Static Assertions)
To ensure that Rust's type system still strictly enforces the user-defined constraints (e.g., `K: Hash + Eq`), the trait bounds are **not discarded** during monomorphization. Instead, they are transformed into concrete static assertions.

During the AST traversal, the walker also visits the bounds extracted from `syn::Generics`. It substitutes the generic placeholders with the inferred concrete types and appends them to the generated function as a `where` clause.

**Original DSL:**
```rust
rule key_value_map<K: Hash + Eq, V>(k_parser, v_parser) -> HashMap<K, V> = ...
```

**Generated Concrete Output (Passed to Backends):**
```rust
// The generic parameters are stripped from the signature...
fn __key_value_map_string_i32(input: ParseStream) -> Result<HashMap<String, i32>>
// ...but the trait bounds are preserved as concrete type assertions!
where
    String: Hash + Eq, 
{
    // Concrete Action Block
}
```
If `String` did not implement `Hash`, `rustc` would immediately halt compilation at this exact `where` clause, preserving the exact safety guarantees the user requested.

### 1.4. Backend Lowering
The backends (`syn-grammar`, `winnow-grammar`) receive the monomorphized, concrete rules. They do not need to process any generic logic. They simply desugar the AST into their optimized target structures (e.g., translating a `[T]` return type into a `winnow::combinator::separated` block or a `syn` `while` loop).

---

## 2. Consequences

### Advantages
* **Zero-Cost Verification:** By translating trait bounds into concrete `where` clauses, the macro defers the actual type-checking to `rustc`, ensuring mathematically proven trait verification without writing a custom type-checker in the macro.
* **Left-Recursion Safety:** Because all higher-order rules are flattened into standard concrete rules at macro-expansion time, the static call graph remains fully visible. The automatic left-recursion elimination algorithm functions perfectly without modification.
* **Unified Frontend Parsing:** Utilizing `syn::Generics` allows the `syn-grammar-model` to rely entirely on battle-tested parsing infrastructure.
* **Backend Simplicity:** Backends are relieved of all generic resolution logic, allowing them to focus strictly on emitting the most performant parsing combinators for their specific domain.

### Risks and Mitigations
* **Binary Bloat (Code Duplication):** Monomorphization generates a distinct Rust function for every unique instantiation of a generic rule. This is deemed an acceptable trade-off to maintain static call graph visibility, closely mirroring `rustc`'s own generic monomorphization behavior.
* **Nested Resolution Complexity:** Deeply nested generic invocations (e.g., `list(list(integer))`) require recursive template substitution. The AST walker mitigates infinite loops by enforcing a strict expansion depth limit.
