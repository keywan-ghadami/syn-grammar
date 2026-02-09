# Extending syn-grammar as a Frontend

`syn-grammar` is designed to be a modular frontend for defining grammars in Rust. While it provides a default backend targeting `syn` types and recursive descent parsing, its components are decoupled to allow the creation of alternative backends (e.g., `winnow-grammar`, `chumsky-grammar`).

This guide explains how to use `syn-grammar`'s infrastructure to build your own parser backend or extend the existing one.

## Architecture & Reusable Components

The project is split into several crates, each providing reusable functionality:

### 1. `syn-grammar-model` (The Frontend)
This crate handles the syntax and semantics of the grammar definition language itself. Backends can use this to parse and validate grammars without worrying about the specific syntax.
-   **Parsing**: Parses the `grammar! { ... }` block into a structured `GrammarDefinition`.
-   **Validation**: checks for undefined rules, unused rules, and other common errors.
-   **Analysis**: Contains powerful analysis tools, such as:
    -   **Left Recursion Detection**: Identifies direct and indirect left recursion loops.
    -   **Cycle Detection**: Finds infinite loops in the grammar graph.
    -   **First/Follow Sets**: (Future) Helpers for LL(k) analysis.

**Use Case**: If you are writing a backend that compiles grammar definitions to a different target (e.g., a parser combinator library), use `syn-grammar-model` to parse the user's input.

### 2. `grammar-kit` (The Runtime)
This crate provides the runtime components needed by the generated parsers. It is re-exported as `syn_grammar::rt`.
-   **`ParseContext`**: Manages the state of the parser, including error accumulation and recovery.
-   **Error Recovery**: Utilities like `attempt`, `attempt_recover`, and `skip_until` provide robust error handling strategies common to recursive descent parsers.
-   **Backtracking**: Helper functions to try multiple alternatives and backtrack on failure.

**Use Case**: Your backend can reuse `grammar-kit` to handle the tricky parts of parser state management, even if your specific parsing logic differs.

### 3. `syn-grammar-macros` (The Codegen)
This crate contains the logic to generate Rust code from the `GrammarDefinition`.
-   **Default Codegen**: Targets `syn::parse::ParseStream` and produces standard recursive descent functions.
-   **Extensibility**: The built-in rule handling has been decoupled (see below), allowing you to swap out implementations for basic tokens.

### 4. `grammar-kit/testing`
A testing framework designed to verify parsers.
-   **Source Verification**: Ensures that the generated parser can round-trip (parse and print) correctly, or at least match the structure of the input.
-   **Snapshot Testing**: compare parser output against expected ASTs.

## Building a Backend (e.g., `winnow-grammar`)

To build a backend that uses `syn-grammar`'s syntax but targets a different library (like `winnow`), you have two main approaches:

### Approach A: Injection (Reuse Codegen)
If your target library can be adapted to the `fn(input, ctx) -> Result<T>` signature, you can reuse the existing `syn-grammar` codegen and simply inject your own implementations for the leaf rules (terminals).

**How it works:**
1.  Create a library crate (e.g., `winnow-adapters`) that implements the core rules (`ident`, `string`, `integer`) using `winnow`.
2.  Ensure these functions match the `syn-grammar` signature.
3.  Users of your backend simply import your adapters in their grammar.

**Example:**
```rust
// User's code
use winnow_adapters::*; // Injects 'ident', 'string', etc.

grammar MyParser {
    rule main = ident  // Uses winnow_adapters::parse_ident_impl
}
```

This approach is best if you want to keep the overall structure of recursive descent functions but change how tokens are consumed.

### Approach B: Custom Codegen
If the target architecture is fundamentally different (e.g., a parser combinator struct instead of functions), you should write a custom macro.

**How it works:**
1.  Depend on `syn-grammar-model`.
2.  Create a proc-macro that parses the input using `syn_grammar_model::parse_grammar`.
3.  Perform your own code generation by iterating over the `GrammarDefinition`.
4.  (Optional) Reuse `syn_grammar_model::analysis` to handle left recursion or optimize your output.

This gives you complete control over the generated code structure.

## Extending the Default Backend

`syn-grammar` provides a set of built-in rules (like `ident`, `string`, `lit_int`) that map to `syn` types by default. These are now implemented as default functions in `syn_grammar::builtins` and are automatically imported.

This decoupling allows you to override them easily.

### How Built-ins Resolution Works
When a rule calls `ident`, the generated code calls `parse_ident_impl`. This resolves in the following order:
1.  **Local Rule**: A `rule ident` defined in the grammar block.
2.  **Imported Item**: A function imported via `use my_mod::ident;`.
3.  **Built-in**: The default `syn_grammar::builtins::ident` (wildcard import).

### Overriding Built-ins
To replace a built-in rule with your own logic (or a backend's logic):

**1. Define it locally:**
```rust
grammar MyGrammar {
    rule ident -> MyType { ... } // Shadows built-in
}
```

**2. Import it (Injection):**
```rust
grammar MyGrammar {
    use my_backend::parse_ident_impl; // Shadows built-in wildcard
    rule list = ident*
}
```

**3. Resolve Ambiguity:**
If you inherit from a grammar that provides `ident` AND the built-ins provide `ident`, usage is ambiguous. Resolve it by explicitly importing the one you want:
```rust
grammar Child : Parent {
    use super::Parent::parse_ident_impl;
    rule usage = ident
}
```
