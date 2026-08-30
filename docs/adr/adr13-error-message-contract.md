# ADR 13: Fehlermeldungs-Vertrag

## Status

Accepted. Dies ist der verbindliche Anforderungskatalog aus [`GOALS.md`](../../GOALS.md).

Verhältnis zu den bestehenden ADRs: ADR-09, ADR-11 und ADR-12 beschreiben **Mechanik**
(strukturierter Fehlerzustand, Abstraktion der Fehlerauslösung, Aggregation). Dieses ADR
beschreibt das **beobachtbare Ergebnis**. Bei Widerspruch gilt dieses ADR, denn es ist
durch Tests belegt.

## Context

Die Anforderungen an Fehlermeldungen standen bisher implizit in neun Testdateien.
Dadurch war „Enterprise-Niveau" nicht überprüfbar: Es gab keinen Ort, an dem stand, was
eine gute Meldung ausmacht, und keinen Weg zu entscheiden, ob eine Änderung eine
Verbesserung oder eine Regression ist.

Alle Anforderungen unten sind aus vorhandenen Assertions abgeleitet und mit Fundstelle
belegt. Was hier nicht steht, ist keine Anforderung.

Verglichen wird per **Substring** (`core/grammar-kit/src/testing.rs:199-212`,
`assert_failure_contains`). Mehrzeilige Erwartungen sind zusammenhängende Substrings
inklusive `\n` — die Reihenfolge der `in …`-Zeilen ist damit Vertragsbestandteil, das
Ende der Meldung nicht.

## Decision

### 1. Erwartung benennen

Jede Meldung sagt, was erwartet wurde. Tokens in Backticks, Primitive ohne.

* `expected \`c\`` — `error_reporting_test.rs:65`
* `expected identifier` — `list_dx_test.rs:50`
* `expected \`,\`` — `list_dx_test.rs:82`

### 2. Labels ersetzen die Token-Ebene

Ein explizites Label (`# "…"` an einer Variante, `item_label=` bei Listen) tritt an die
Stelle der internen Token-Erwartung.

* `expected \`type name\`` statt `expected \`::\`` — `list_dx_test.rs:40`
* `expected one of: \`Letter A\`, \`Letter B\`` — `labeled_alternatives_test.rs:47`

### 3. Gefundenes benennen

* `; found unexpected token \`123\`` — `list_dx_test.rs:30`
* `unexpected end of input, …` — `list_dx_test.rs:72`, `trailing_comma_test.rs:41`
* `unexpected end of group, …` — `list_dx_test.rs:60`
* `unexpected match for rule \`bad\`; found \`bad\` in rule \`main\`` — `peek_not_test.rs:87`

### 4. Position

Format `at column N (line M)`, genau einmal in der Meldung.

**Einschränkung, bindend:** Diese Angabe wird nur ausgegeben, wenn der Span echte
Positionsdaten trägt. Auf stable Rust liefert ein Prozedurmakro `(0,0)`
(`proc-macro2-1.0.106/src/wrapper.rs:449-450`); dort wird die Angabe **weggelassen**
statt als `0` gedruckt — rustc unterstreicht den Span ohnehin selbst.

Die Positionsangabe dient allein der **Anzeige**. Für die **Auswahl** des besten Fehlers
ist sie unbrauchbar; dafür gilt Punkt 8.

### 5. Regel-Stack

Mehrzeilig, von innen nach außen, dedupliziert, Regelnamen in Leerzeichen-Form
(`deepest_err` → `deepest err`).

```
expected `c` at column 4 (line 1)
in deepest err
in main
```
— `error_reporting_test.rs:65`, ebenso `:97` (`in inner rule`).

Verschachtelung über mehrere Ebenen: `list_dx_test.rs:40`
(`in type name` → `in param` → `in function parameter 1` → `in signature`).

### 6. Alternativen-Aggregation

Scheitern mehrere Alternativen an derselben Stelle, entsteht **eine** Meldung:
`expected one of: …`, sortiert und dedupliziert.

* `expected one of: \`a\`, \`b\`` — `labeled_alternatives_test.rs:39`
* `expected one of: \`one\`, \`two\`, \`zero\`` (alphabetisch) — `error_reporting_test.rs:81`
* auch innerhalb von Gruppen — `labeled_alternatives_test.rs:66`

### 7. Tiefe schlägt Aggregation

Kam der Parser in einer Alternative weiter, verdrängt dieser Fehler die Aufzählung —
`expected one of:` darf dann **nicht** erscheinen.
— `labeled_alternatives_test.rs:57-58` (prüft beides, Positiv- und Negativfall)

### 8. Auswahlreihenfolge

Bei konkurrierenden Fehlern entscheidet, in dieser Reihenfolge:

1. **Fatalität** — ein Fehler hinter einem Cut (`=>`) gewinnt immer
2. **Fortschritt** — wer weiter im Input kam, gewinnt.
   Gemessen am Cursor über `PartialOrd for Cursor` (syn 2.0.114, `src/buffer.rs:401-409`),
   **nicht** an Zeile/Spalte (siehe Punkt 4)
3. **Priorität** — `fail` > Label > Standard
4. **Kontext-Spezifität** — tieferer Regel-Stack oder vorhandenes Label

Die Länge der Nachricht ist **kein** Kriterium (ADR-09 nennt sie als Instabilitätsquelle).

Belegt in `error_abstraction_test.rs:124` (Tiefe schlägt `fail`-Priorität) und `:136`
(bei gleicher Stelle gewinnt `fail`).

### 9. Cut

`=>` unterdrückt Fehler aller späteren Alternativen vollständig.
— `error_abstraction_test.rs:30,88,95`, `fail_test.rs:38`

### 10. `fail("msg")`

Der Text erscheint wortwörtlich, ohne `expected`-Präfix und ohne Auto-Label.

* `zero is not allowed` — `error_abstraction_test.rs:57`
* `hard fail` — `error_abstraction_test.rs:136`
* `foo cannot be followed by bar` — `fail_test.rs:38`

### 11. Listen-Diagnostik

* Item-Index im Stack: `in function parameter 2` (`list_dx_test.rs:50`),
  `in item 3` (`trailing_comma_test.rs:41`), `in function argument 3` (`list_test.rs:159-161`)
* Separator-Kontext: `in separator` — `list_dx_test.rs:82`
* Mindestanzahl mit Ist-Wert: `expected at least 2 items, found 1` — `list_test.rs:112`

### 12. Fehler aus Action-Blöcken

Ein vom Nutzer im Action-Block erzeugter `syn::Error` wird unverfälscht weitergereicht und
nur um Position und Regel-Stack angereichert — nicht mit `expected …` überschrieben.

```
expected 'a' at column 4 (line 1)
in inner
in outer
```
— `error_reporting_test.rs:152`. **Heute nicht erfüllt**; im Test explizit als Sollzustand
markiert (`:149-151`).

### 13. Lazy Formatting

Die Nachricht wird während des Parsens **nie** verändert. Regelnamen, Labels und Position
werden erst beim Übergang nach `syn::Error` zusammengesetzt. Das hält die Auswahl aus
Punkt 8 von der Textgestalt unabhängig und macht Ergebnisse deterministisch.
— ADR-09

### 14. Nachweis auf dem echten Makro-Pfad

Mindestens ein Test muss die Meldung über den **Prozedurmakro-Pfad** prüfen, nicht über
`parse_str`. Sonst bleibt Punkt 4 unbemerkt verletzt: `parse_str` benutzt den Fallback von
proc-macro2 und liefert echte Positionen, ein reales Makro auf stable nicht.
Vorgesehen als `trybuild`-Fall unter `syn-grammar/tests/ui/`.

## Consequences

* „Enterprise-Niveau" ist ab hier messbar: Punkte 1-14 sind erfüllt oder nicht.
* Punkte 12 und 14 sind heute offen und damit benannte Lücken statt unsichtbarer Mängel.
* Punkt 8 verlangt, die Auswahl von `span.start()` auf den Cursor umzustellen. Das ist eine
  Verhaltensänderung im Kern und der Grund, warum die Diagnostik überhaupt neu gebaut wird.
