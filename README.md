
# Grammar Ecosystem

The **Grammar** ecosystem is a powerful parser generator framework for Rust. It allows you to define EBNF-like grammars directly inside your Rust code using a macro, which are then compiled into efficient parsers at compile time.

## Backends

The framework supports multiple backends depending on your parsing needs:

- **[`syn-grammar`](syn-grammar/README.md)**: Designed for parsing Rust TokenStreams. Ideal for writing procedural macros, DSLs embedded in Rust, and code generation tools.
- **[`winnow-grammar`](winnow-grammar/README.md)**: Built on top of `winnow`. Designed for general-purpose parsing of text (`&str`) and binary data (`&[u8]`). Ideal for file formats, protocols, and configuration files.

## Documentation

- **[Ziele](GOALS.md)**: Was das Projekt sein will, und die Randbedingungen, die daraus folgen.
- **[Architektur](ARCHITECTURE.md)**: Wie der Code heute tatsächlich aufgebaut ist.
- **[Fehlermeldungs-Vertrag](docs/adr/adr13-error-message-contract.md)**: Der verbindliche
  Anforderungskatalog an Diagnosemeldungen.
- **[Architekturentscheidungen](docs/adr/)**: ADRs.
- **[Extending Guide](EXTENDING.md)**: Für Autoren eigener Backends. **Veraltet** — beschreibt
  eine API, die es nicht mehr gibt.

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
