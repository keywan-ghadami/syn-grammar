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

Auf **stable Rust** liefert `proc_macro2::Span::start()` innerhalb eines Prozedurmakros
keine Positionsdaten. Nachgewiesen in `proc-macro2-1.0.106/src/wrapper.rs:449-450`:

```rust
#[cfg(not(proc_macro_span_location))]
Span::Compiler(_) => LineColumn { line: 0, column: 0 },
```

Jede Diagnostik, die zur **Auswahl** des besten Fehlers Zeilen- oder Spaltenangaben
vergleicht, ist im echten Produkteinsatz wirkungslos — auch wenn die Tests grün sind,
weil `parse_str` den Fallback-Pfad von proc-macro2 benutzt.

Daraus folgt bindend:

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

* **Keine Harmonisierung mit `winnow-grammar`.** `winnow-grammar` wird ein eigenes,
  unabhängiges Projekt. In diesem Repository wird daran nur so viel getan, wie die
  spätere Abtrennung erfordert — keine Feature-Arbeit.
* **Keine gemeinsame Codegen-Abstraktion** über beide Backends. Die beiden
  Codegeneratoren bleiben getrennt, bis winnow ausgezogen ist.

## Offene Entscheidung

`winnow-grammar` benutzt heute das gemeinsame Frontend `core/syn-grammar-model` (DSL-Parser,
Modell, Validator). Beim Auszug ist zu entscheiden, ob dieses Crate eigenständig
veröffentlicht wird (dann mit backend-neutralem Namen) oder ob `winnow-grammar` es forkt.
Das ist noch nicht entschieden.
