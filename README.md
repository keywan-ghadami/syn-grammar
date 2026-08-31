
# Grammar Ecosystem

The **Grammar** ecosystem is a powerful parser generator framework for Rust. It allows you to define EBNF-like grammars directly inside your Rust code using a macro, which are then compiled into efficient parsers at compile time.

## Umfang

Dieses Repository enthält **[`syn-grammar`](syn-grammar/README.md)**: einen
Parser-Generator für Rust-TokenStreams, gedacht für prozedurale Makros, in Rust
eingebettete DSLs und Codegeneratoren.

Das frühere zweite Backend **`winnow-grammar`** (allgemeines Parsen von Text und
Binärdaten auf Basis von `winnow`) ist seit dem 31.08.2026 ein eigenständiges
Projekt: <https://github.com/keywan-ghadami/winnow-grammar>. Es hat das Frontend
(DSL-Parser, Modell, Validator) beim Auszug geforkt; beide Fassungen der
Grammatik-Sprache entwickeln sich ab Commit `64be1ef` unabhängig weiter.

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
