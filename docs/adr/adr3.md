# Architecture Decision Record (ADR) 003: Higher-Order Generic Rules, Macro-Time Monomorphization, and Trait Bound Preservation

**Date:** 2026-02-15 (Revised: 2026-02-22)
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
* **Rule Arguments (`k_parser`, `v_parser`):** Left untyped in the DSL signature. The macro implicitly treats these arguments as substitutable AST nodes (sub-rules or inline patterns).
* **Return Type inference (`-> [T]`):** For simple lists, the DSL supports `[T]` (parsed as `syn::Type::Slice`), which acts as syntactic sugar for `Vec<T>`.
* **Invocation (Inferred) [Added 2026-02-22]:** `key_value_map<_>(string_ident, integer)`
* **Invocation (Explicit) [Added 2026-02-22]:** `key_value_map<String, i32>(string_ident, integer)`

### 1.2. The Resolution Engine (Macro-Time Monomorphization)
When a generic rule is invoked, the frontend performs a deterministic AST transformation before passing the syntax tree to the backends:

1. **Path & Inference Parsing:** The parser identifies `key_value_map<_>` natively via `syn` as a path with an inference argument.
2. **Registry Lookup:** Identifies the target arguments (`string_ident`, `integer`) and extracts their previously registered return types (e.g., `String`, `i32`).
3. **Type Inference & Binding:** Positively matches the generic parameters to the resolved types: `K = String`, `V = i32`.
4. **Template Cloning & Substitution:** A `syn::visit_mut::VisitMut` walker deep-copies the AST of the sub-rule, replaces all instances of `K` and `V` with the inferred types, and substitutes the untyped parameter invocations with direct calls to the concrete rules.

### 1.3. Trait Bound Preservation (Static Assertions)
To ensure that Rust's type system still strictly enforces the user-defined constraints (e.g., `K: Hash + Eq`), the trait bounds are **not discarded** during monomorphization. Instead, they are transformed into concrete static assertions.

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
If `String` did not implement `Hash`, `rustc` would immediately halt compilation at this exact `where` clause.

### 1.4. Backend Lowering
The backends (`syn-grammar`, `winnow-grammar`) receive the monomorphized, concrete rules. They do not need to process any generic logic. They simply desugar the AST into their optimized target structures.

---

## 2. Consequences

### Advantages
* **Zero-Cost Verification:** Defers actual type-checking to `rustc` via concrete `where` clauses, ensuring mathematically proven trait verification.
* **Left-Recursion Safety:** Flattening higher-order rules at macro-expansion time keeps the static call graph fully visible.
* **Unified Frontend Parsing:** Utilizing `syn::Generics` and `syn::Path` allows the toolchain to rely entirely on battle-tested parsing infrastructure.

### Risks and Mitigations
* **Binary Bloat (Code Duplication):** Monomorphization generates a distinct Rust function for every unique instantiation. This mirrors `rustc`'s own generic behavior and is an acceptable trade-off.
* **Nested Resolution Complexity:** Deeply nested generic invocations require recursive template substitution, mitigated by enforcing a strict expansion depth limit.

---

## Appendix: Revision 2026-02-22 (Syntactic Disambiguation & The `<_>` Marker)

**Context for Revision:** During the initial implementation of the `syn-grammar-model` parser, a fundamental LL(1) parsing ambiguity was discovered. Parsing EBNF structures with standard Rust macro toolchains (`syn`) causes sequencing conflicts. The sequence `rule (param)` is natively parsed by `syn` as an Identifier (`Ident`) followed by a Token Group (`Group`), making it impossible to reliably distinguish a parameterized rule invocation from a standard EBNF grouping without expensive or fragile custom lookaheads.

**The Solution:**
To resolve this without breaking `syn`'s capabilities, an explicit token marker is required before the opening parenthesis for rule invocations. The `<_>` syntax was adopted for cases where types are implicitly inferred.

### Rationale for the `<_>` Syntax
1. **Native `syn` Integration:** `<_>` is automatically parsed by `syn` as a `syn::Path` with `AngleBracketedGenericArguments`. It sidesteps the LL(1) limitation entirely without custom parsing logic.
2. **Semantic Consistency:** In Rust, the underscore `_` explicitly represents "Type Inference". Since the macro backend physically infers the types from the provided arguments during monomorphization (as defined in the original ADR), the syntax accurately reflects the underlying architectural behavior.
3. **Scalability:** Transitioning from inferred types (`<_>`) to explicit types (`<String, i32>`) requires zero changes to the internal parser structure or the user's mental model.

### Evaluated and Rejected Alternatives

#### 1. Macro Syntax: `rule!(param)`
* **Concept:** Using the exclamation mark to signal an invocation.
* **Reason for Rejection:** Terrible ergonomics and parser compatibility when combined with generics. The syntax `rule!<T>(param)` does not exist in standard Rust. `syn` cannot parse this sequence natively, forcing the implementation of a highly fragile, custom parser operating on raw token streams.

#### 2. Bracketed Arguments: `rule[param]` or `rule<T>[param]`
* **Concept:** Shifting the parameter payload into square brackets to avoid conflicts with round EBNF parentheses.
* **Reason for Rejection:** Square brackets `[...]` are already strictly reserved and in use for other semantic constructs within the `syn-grammar` ecosystem. Overloading this token would introduce severe parser ambiguities elsewhere.

#### 3. Empty Generics: `rule<>(param)`
* **Concept:** A visual generic marker without the underscore.
* **Reason for Rejection:** Empty angle brackets attached to identifiers are invalid Rust syntax. The standard `syn::Path` parser fails immediately upon encountering `<>`. Supporting this would require bypassing `syn`'s robust path parsing infrastructure entirely.
