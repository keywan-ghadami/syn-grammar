# syn-grammar

**syn-grammar** is a parser generator for Rust token streams. You define an
EBNF-like grammar inside your Rust code with the `grammar!` macro, and it is
compiled into an efficient `syn` parser at compile time. It is aimed at
procedural macros, DSLs embedded in Rust, and code generators.

```rust
grammar! {
    grammar Calc {
        pub rule expression -> i32 =
            l:expression "+" r:term -> { l + r }
          | t:term                  -> { t }

        rule term -> i32 = i:i32 -> { i }
    }
}
```

**Start here: [`syn-grammar/README.md`](syn-grammar/README.md)** — installation,
quick start, built-ins and the testing API.

## Repository layout

This is a Cargo workspace; the crate users depend on is `syn-grammar`.

| Path | Crate | What it is |
|---|---|---|
| [`syn-grammar/`](syn-grammar/) | `syn-grammar` | The public crate: the `grammar!` macro and the `syn` built-ins. |
| [`syn-grammar/syn-grammar-macros/`](syn-grammar/syn-grammar-macros/) | `syn-grammar-macros` | Code generation. Internal; re-exported by `syn-grammar`. |
| [`core/syn-grammar-model/`](core/syn-grammar-model/) | `syn-grammar-model` | DSL parser, semantic model and validation — shared by all backends. |
| [`core/grammar-kit/`](core/grammar-kit/) | `grammar-kit` | Runtime and testing utilities used by generated parsers. |
| [`core/grammar-kit-macros/`](core/grammar-kit-macros/) | `grammar-kit-macros` | Derive macros for `grammar-kit`. |
| [`cxx-parser/`](cxx-parser/) | `cxx-parser` | Dogfooding stress test against a CXX-like IDL. Not published. |

For general-purpose parsing of text and binary data there is a sibling project,
[`winnow-grammar`](https://github.com/keywan-ghadami/winnow-grammar), built on
[`winnow`](https://docs.rs/winnow). It shares the grammar DSL but is developed
separately; neither crate depends on the other.

## Documentation

- **[Grammar syntax reference](syn-grammar/SYNTAX.md)** — the grammar definition
  language: rules, operators, built-ins, error messages.
- **[Goals](GOALS.md)** — what the project is for, and the constraints that
  follow from that.
- **[Architecture](ARCHITECTURE.md)** — how the code is actually laid out today.
- **[Error message contract](docs/adr/adr13-error-message-contract.md)** — the
  binding requirements for diagnostics. Error quality is the quality criterion
  of this project, so this is the document to read first when changing the
  runtime.
- **[Error handling](docs/ERROR_HANDLING.md)** — how the engine selects and
  renders the error it reports.
- **[Architecture decisions](docs/adr/)** — the ADRs.
- **[Limitations](docs/LIMITATIONS.md)** — what this design cannot do, and why.

## Building

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Those are the checks the CI runs; the exact steps are in
[`.github/workflows/ci.yaml`](.github/workflows/ci.yaml).

## License

Licensed under either of

- Apache License, Version 2.0 ([`LICENSE-APACHE`](LICENSE-APACHE))
- MIT license ([`LICENSE-MIT`](LICENSE-MIT))

at your option.
