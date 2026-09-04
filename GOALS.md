# Ziele

Dieses Dokument hält fest, was das Projekt sein will. Es ist die Referenz, gegen die
Architektur- und Umsetzungsentscheidungen geprüft werden. Es ersetzt `ARCHITEKTUR_MANIFEST.txt`
und `PROJECT_STRUCTURE.md`, die maschinell erzeugt wurden und Code beschreiben, den es
teilweise nicht mehr gibt.

Stand: 2026-08-30.

## Was das Produkt ist

**`syn-grammar` ist ein Parser-Generator für Rust-Prozedurmakros.** Aus einer
EBNF-ähnlichen Grammatik (`grammar! { … }`) entsteht Rust-Code, der einen
`proc_macro2::TokenStream` parst.

Das ist der Fokus der Weiterentwicklung.

## Das eigentliche Qualitätsmerkmal: Fehlermeldungen

Ein Parser-Generator ist so gut wie seine Fehlermeldungen. Sie sind kein Beiwerk,
sondern der Grund, einen Generator einer handgeschriebenen `syn`-Schleife vorzuziehen.

Der verbindliche Anforderungskatalog steht in
[`docs/adr/adr13-error-message-contract.md`](docs/adr/adr13-error-message-contract.md).
Was dort nicht steht, ist keine Anforderung; was dort steht, ist durch Tests belegt.

## Randbedingung, die die Architektur bestimmt

**Stand 31.08.2026: teilweise entschärft — die Folgerungen gelten weiterhin.**

Bis Rust 1.87 lieferte `proc_macro2::Span::start()` innerhalb eines Prozedurmakros
keine Positionsdaten:

```rust
#[cfg(not(proc_macro_span_location))]
Span::Compiler(_) => LineColumn { line: 0, column: 0 },
```

Seit **Rust 1.88** setzt proc-macro2 dieses `cfg` auch auf stable
(`proc-macro2/build.rs`: `rustc >= 88 && compile_probe_stable("proc_macro_span_location")`),
und `Span::start()` liefert echte Zeilen und Spalten. Belegt durch
`syn-grammar/tests/ui/runtime_error_real_macro.stderr` — ein Schnappschuss aus einem
echten Makro, mit Positionsangabe.

Das Projekt setzt deshalb `rust-version = "1.88"`; ältere Toolchains weist cargo mit
klarer Meldung ab, und ein eigener CI-Job baut gegen genau diese Version.

**Die Folgerungen bleiben trotzdem bindend**, aus zwei Gründen: die Cursor-Metrik ist
mit O(1) ohnehin billiger als ein Positionsvergleich, und sie hängt an gar keiner
Toolchain-Eigenschaft. Ein Verhalten, das erst ab einer bestimmten Compilerversion
korrekt wird, ist kein gutes Fundament für das Qualitätsmerkmal dieses Projekts.

* **Auswahl** (welcher Fehler gewinnt) benutzt eine toolchain-unabhängige
  Fortschrittsmetrik. `syn::buffer::Cursor` implementiert `PartialOrd` (Zeigervergleich
  im gemeinsamen `TokenBuffer`, O(1)) — das ist die Metrik.
* **Anzeige** ist davon getrennt. Im Prozedurmakro unterstreicht rustc den Span selbst;
  eine Textangabe `at column N` ist dort wertlos und wird weggelassen statt als `0` gedruckt.
* Es muss mindestens einen Test geben, der den **echten Makro-Pfad** prüft, nicht nur
  `parse_str`.

## Abnahme-Benchmark

`cxx-parser` ist der konkrete Anwendungsfall, an dem sich das Projekt messen lässt: eine
Fremd-DSL, die ohne trennende Delimiter in echte Rust-Syntax übergeht. Er muss fehlerfrei
funktionieren und erstklassige Fehlermeldungen liefern.

`cxx-parser` bleibt auf dem syn-Backend.

## Nicht-Ziele

* **Keine Harmonisierung mit `winnow-grammar`.** Das ist ein eigenständiges
  Projekt unter <https://github.com/keywan-ghadami/winnow-grammar>. Es hat das
  Frontend (DSL-Parser, Modell, Validator) geforkt statt es zu beziehen — das
  Modell hier ist `syn`-basiert (`syn::Path`, `syn::Lit`, `syn::Type`) und lässt
  sich nicht backendneutral weiterentwickeln. Die beiden Fassungen der DSL
  driften also auseinander. Dieses Repository enthält nur das syn-Backend.
* **Keine gemeinsame Codegen-Abstraktion** über mehrere Backends.

