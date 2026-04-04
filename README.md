1
# Grammar Ecosystem

The **Grammar** ecosystem is a powerful parser generator framework for Rust. It allows you to define EBNF-like grammars directly inside your Rust code using a macro, which are then compiled into efficient parsers at compile time.

## Backends

The framework supports multiple backends depending on your parsing needs:

- **[`syn-grammar`](syn-grammar/README.md)**: Designed for parsing Rust TokenStreams. Ideal for writing procedural macros, DSLs embedded in Rust, and code generation tools.
- **[`winnow-grammar`](winnow-grammar/README.md)**: Built on top of `winnow`. Designed for general-purpose parsing of text (`&str`) and binary data (`&[u8]`). Ideal for file formats, protocols, and configuration files.

## Documentation 
- **[Extending Guide](EXTENDING.md)**: Guide for library authors on how to build custom backends using `syn-grammar-model`.

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
