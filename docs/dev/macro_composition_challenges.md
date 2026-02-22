# Macro Composition Challenges in Rust

This document details the technical challenges and constraints encountered when implementing macro-based grammar composition in Rust, specifically regarding visibility, re-exports, and recursive macro invocations across modules.

## The Goal

We want to allow users to define grammars in separate modules (or even crates) and compose them using an `include` directive. The `include` directive should allow a grammar to inherit rules from another grammar.

Example:
```rust
// In module A
grammar! { grammar A { ... } }

// In module B
grammar! { include A_rules as a; grammar B { ... } }
```

## The Implementation Strategy

The `grammar!` macro generates a companion macro (e.g., `A_rules`) that encapsulates the rules of grammar A. When `B` includes `A`, `B_rules` macro invokes `A_rules` to "collect" the rules of A into a shared accumulator. This accumulator is then used to generate the parser for B.

This requires `A_rules` to be:
1.  **Exported** from module A so B can access it.
2.  **Recursively composable** (A might include other grammars).

## Constraints Encountered

### 1. `#[macro_export]` vs Module Visibility

Rust macros defined with `macro_rules!` behave differently depending on whether they are `#[macro_export]`ed or not.

*   **`#[macro_export]`**: The macro is exported at the **crate root**. It ignores the module hierarchy.
    *   *Pros*: Accessible from anywhere in the crate (and other crates) via the crate root.
    *   *Cons*:
        *   **Absolute Path Restriction**: In the *same* crate, you cannot refer to `#[macro_export]` macros using absolute paths like `crate::A_rules` due to a rustc limitation ("macro-expanded `macro_export` macros from the current crate cannot be referred to by absolute paths"). You must refer to them by simple name `A_rules` if they are in scope.
        *   **Naming Conflicts**: Since they are all flat at the crate root, names must be unique across the entire crate.

*   **Local Macros (without `#[macro_export]`)**: These are scoped to the module.
    *   *Pros*: Respect module hierarchy. Can be `pub use`d or `pub(crate) use`d to control visibility.
    *   *Cons*: Not automatically available everywhere. Must be imported.

**Decision**: We switched to generating local macros with `pub(crate) use` (or `pub use`). This allows us to refer to them using standard module paths (e.g., `crate::module_a::A_rules`), avoiding the "absolute path" error.

### 2. Recursive Macro Invocations and Scoping

The generated `_rules` macro needs to be recursive to handle the chain of includes.
Originally, the macro looked like this:

```rust
macro_rules! A_rules {
    ($alias, ...) => {
        // Recursive step: call self with initial state
        A_rules! { @accum ... }
    }
}
```

The issue arises when `A_rules` is imported via an alias or path (e.g., `crate::mod_a::A_rules`).
When `A_rules` is invoked, it expands to a call to `A_rules!`.
However, `A_rules` (the identifier) might **not be in scope** at the call site (e.g., inside `mod_b`). Only the path `crate::mod_a::A_rules` works.

If the macro expands to call itself by simple name, it breaks unless the user also imports the macro by its exact name.

**Solution**:
We modified the macro generation to **avoid self-recursion by name** in the entry branch. Instead of calling itself, the entry branch duplicates the logic of the initial step or calls the dependency chain directly.

### 3. Dependency Visibility

If grammar `B` includes grammar `A`, `B_rules` generates code that calls `A_rules`.
When grammar `C` includes `B`, it calls `B_rules`. `B_rules` expands in `C`'s context.
`B_rules` then calls `A_rules`.

This means `A_rules` **must be visible** in `C`'s context.
If `B` used `include A_rules` (simple identifier), `B_rules` generates `A_rules!`.
This requires `C` to import `A_rules`, even though `C` only knows about `B`. This leaks implementation details.

**Constraint**: All includes must use **fully qualified paths** (or paths relative to crate root) so that the generated code works from any context.
Example: `include crate::mod_a::A_rules as a;`.

This ensures that `B_rules` generates `crate::mod_a::A_rules!`, which resolves correctly anywhere in the crate.

### 4. `no rules expected ::` Error

We encountered obscure errors like `no rules expected ::` when invoking macros with paths (e.g. `crate::mod::Macro!`). This was likely due to the recursive definition issue mentioned in (2) combined with how `macro_rules!` handles path invocations when the macro expects to call itself. Using the "duplicate entry branch" strategy (removing self-recursion) seems to solve this.

## Architectural Implications and Recommendations

The current macro composition approach creates a strong coupling between the grammar definition and the visibility of its dependencies. It forces users to be very aware of module paths and imports.

**Proposal: Treat External Grammars as Black Boxes**

If we relax the requirement for "white-box" composition (inheriting individual rules and extending them), we could simplify the system.

*   **Current (White-box)**: `grammar B` includes `A`. `B` can override rules of `A` or use them in new rules. The final parser for `B` contains code for all rules of `A` and `B` combined. This allows full optimization and validation (e.g., checking for gaps or left-recursion across boundaries).
*   **Proposed (Black-box)**: `grammar B` uses `A` as an external parser. `B` calls `A::parse_rule()` as if it were a built-in token or function.

**Benefits of Black-box approach:**
1.  **Decoupling**: `B` doesn't need to know about `A`'s internal macro structure or dependencies. It just needs `A`'s public API (the parser function).
2.  **Simplicity**: No need for complex recursive macros or visibility hacks. `use A;` is enough.
3.  **Compilation speed**: `A` is compiled once. `B` is compiled separately. In white-box, `A`'s code is effectively duplicated into `B`, increasing macro expansion size.

**Drawbacks:**
1.  **Validation**: We lose global validation. If `B` calls `A`, we can't detect left-recursion loops that cross the boundary (e.g., `A` calls `B` back).
2.  **Optimization**: Cross-grammar optimization is impossible.
3.  **Tokenization**: `A` and `B` might use different tokenization strategies or skip logic, potentially leading to inconsistencies at the boundary.

**Developer Perspective:**
The shift to black-box composition (or "external rule calls") would significantly reduce the complexity of the macro system and making the `include` feature much more robust and easier to use. The current "include" mechanism is fragile due to Rust's macro visibility rules. Unless "inheriting and modifying" a grammar is a core requirement, treating external grammars as libraries (black boxes) is a more standard and stable approach in Rust.
