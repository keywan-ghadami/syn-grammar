# Grammar Ecosystem

The **Grammar** ecosystem is a parser generator framework for Rust. You define
an EBNF-like grammar inside your Rust code with a macro, and it is compiled into
an efficient parser at compile time.

## Scope

This repository contains **[`syn-grammar`](syn-grammar/README.md)**: a parser
generator for Rust token streams, aimed at procedural macros, DSLs embedded in
Rust, and code generators.

For general-purpose parsing of text and binary data there is a sibling project,
[`winnow-grammar`](https://github.com/keywan-ghadami/winnow-grammar), built on
[`winnow`](https://docs.rs/winnow). It shares the grammar DSL but is developed
separately; neither crate depends on the other.

## Documentation

- **[Goals](GOALS.md)** — what the project is for, and the constraints that
  follow from that.
- **[Architecture](ARCHITECTURE.md)** — how the code is actually laid out today.
- **[Error message contract](docs/adr/adr13-error-message-contract.md)** — the
  binding requirements for diagnostics. Error quality is the quality criterion
  of this project, so this is the document to read first when changing the
  runtime.
- **[Architecture decisions](docs/adr/)** — the ADRs.
- **[Limitations](docs/LIMITATIONS.md)** — what this design cannot do, and why.

> Some of these design documents are written in German; the crate
> documentation, the grammar reference and all public API docs are in English.

- **[Extending guide](EXTENDING.md)** — for authors of alternative backends.
  **Outdated**: it describes an API that no longer exists. The header of the
  file says what changed.

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
