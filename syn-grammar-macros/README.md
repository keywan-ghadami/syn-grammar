# syn-grammar-macros

**The code generation engine for `syn-grammar`.**

> **Note:** You should **not** add this crate to your `Cargo.toml` directly. Instead, use the `syn-grammar` crate, which re-exports the macros from this crate.

This crate defines the procedural macros (`grammar!`) that compile the EBNF-like grammar DSL into actual Rust code. While it is an internal implementation detail of `syn-grammar`, understanding its architecture is useful if you intend to write a custom parser backend.

## Responsibilities

1.  **Parsing**: It uses `syn-grammar-model` to parse the raw `TokenStream` from the macro input into a semantic model.
2.  **Validation**: It checks the model for semantic errors (e.g., undefined rules, argument mismatches).
3.  **Code Generation**: It transforms the validated model into a Rust module containing `syn`-based parser functions.

## How it Works

The `grammar!` macro follows a standard compiler pipeline:

1.  **Input**: The macro receives the grammar definition as a `TokenStream`.
2.  **Pipeline**:
    *   `syn_grammar_model::parse_grammar(input)` converts tokens into a `GrammarDefinition` (AST).
    *   `codegen::generate_rust(ast)` traverses the AST and emits Rust code using `quote!`.
3.  **Output**: The resulting `TokenStream` is returned to the compiler, replacing the macro invocation with the generated parser code.

## Creating a Custom Backend

If you want to generate parsers for a different library (e.g., `winnow`, `chumsky`, or a documentation generator) instead of `syn`, you cannot simply "plug in" a generator to this crate. Procedural macros are compiled as separate artifacts, so the code generation logic must be baked into the macro crate itself.

To create a new backend:

1.  **Create a new proc-macro crate** (e.g., `my-grammar-macros`).
2.  **Depend on `syn-grammar-model`**. This gives you the parser for the DSL, so you don't have to rewrite the grammar syntax parsing.
3.  **Implement your own `codegen` module**. This module will take the `GrammarDefinition` from the model and output your desired code.
4.  **Define your own `grammar!` macro**. This is necessary because of a fundamental limitation in Rust procedural macros: a macro crate must contain the logic it executes. You cannot dynamically inject a generator function into an existing compiled macro crate. Therefore, you must define the macro entry point in your own crate to invoke your custom generator.

Example of a custom backend entry point:

```rust
use proc_macro::TokenStream;
use syn_grammar_model::parse_grammar;
// use my_custom_codegen::generate;

#[proc_macro]
pub fn grammar(input: TokenStream) -> TokenStream {
    // 1. Reuse the shared model parser
    let model = match parse_grammar(input.into()) {
        Ok(m) => m,
        Err(e) => return e.to_compile_error().into(),
    };

    // 2. Use your custom generator
    // let output = generate(model);

    // output.into()
    TokenStream::new() // Placeholder
}
```

This architecture ensures that the syntax remains consistent across different backends while allowing complete flexibility in the generated output.
