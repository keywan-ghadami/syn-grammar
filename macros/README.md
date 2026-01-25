# syn-grammar-macros

**Internal implementation detail of `syn-grammar`.**

This crate contains the procedural macro definitions (`grammar!`, `include_grammar!`) used by `syn-grammar`.

## Usage

You should **not** add this crate to your `Cargo.toml` directly. Instead, use the main crate which re-exports these macros and provides the necessary runtime support:

```toml
[dependencies]
syn-grammar = "0.1"
```

See [syn-grammar](https://crates.io/crates/syn-grammar) for documentation and usage examples.
