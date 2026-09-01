# Fehlerbehandlung — Arbeitsweise der Engine

Dieses Dokument erklaert, **wie** die Engine einen Fehler auswaehlt und rendert.

**Was** eine Meldung enthalten muss, steht in
[`adr/adr13-error-message-contract.md`](adr/adr13-error-message-contract.md).
Das ADR ist der verbindliche Katalog; bei Widerspruch gilt das ADR, nicht dieses
Dokument.

> Die frueheren Fassungen dieses Dokuments beschrieben eine Engine, die es nicht
> mehr gibt (Positionsvergleich ueber Zeile/Spalte, Prioritaeten 0/1/2,
> Nachrichtenlaenge als Tiebreak, `!` als Fatal-Marker). Der Text ist gegen
> `core/grammar-kit/src/error.rs` und `context.rs` neu geschrieben.

## Die zentrale Randbedingung

Bis Rust 1.87 lieferte `Span::start()` in einem echten Prozedurmakro fuer jeden
Span `LineColumn { line: 0, column: 0 }`:

```rust
#[cfg(not(proc_macro_span_location))]
Span::Compiler(_) => LineColumn { line: 0, column: 0 },
```

**Seit Rust 1.88 gilt das nicht mehr** — proc-macro2 setzt dieses `cfg` dort auch
auf stable (`build.rs`: `rustc >= 88 && compile_probe_stable(..)`). Das Projekt
verlangt `rust-version = "1.88"`, Positionen sind also verfuegbar. Belegt durch
`syn-grammar/tests/ui/runtime_error_real_macro.stderr`, einen Schnappschuss aus
einem echten Makro.

**Die Trennung von Vergleich und Anzeige bleibt trotzdem.** Nicht aus Not,
sondern weil sie besser ist: der Cursor-Vergleich ist ein Zeigervergleich in
O(1), ein Positionsvergleich waere teurer und haengt an einer
Compiler-Eigenschaft. Ein Verhalten, das erst ab einer bestimmten
Rust-Version korrekt wird, taugt nicht als Fundament fuer die Fehlerqualitaet.

Konsequenz: **Vergleich und Anzeige sind getrennt.**

- **Vergleich** benutzt den `Cursor`. `syn::buffer::Cursor` implementiert
  `PartialOrd` als Zeigervergleich im gemeinsamen `TokenBuffer` — O(1) und
  unabhaengig von Toolchain und Span-Verfuegbarkeit.
- **Anzeige** benutzt den `Span`. Im Prozedurmakro unterstreicht rustc ihn
  selbst im Editor; die Textform `at column N (line M)` wird nur ausgegeben,
  wenn der Span echte Positionsdaten traegt.

## Der Fehlertyp

`core/grammar-kit/src/error.rs`:

```rust
pub struct ParseError<'a> {
    pub span: Span,              // Anzeige
    pub at: Option<Cursor<'a>>,  // Auswahl
    pub message: String,
    pub priority: u8,
    pub is_fatal: bool,
    pub rule_stack: Vec<String>,
}
```

`at` ist `None` nur dort, wo beim Erzeugen kein Cursor zur Hand ist — etwa bei
der Uebernahme eines fremden `syn::Error`. Solche Fehler verlieren jeden
Fortschrittsvergleich gegen einen mit Cursor.

## Welcher Fehler gewinnt — `ParseError::merge`

In genau dieser Reihenfolge:

| Rang | Kriterium | Bemerkung |
|---|---|---|
| 1 | **Fortschritt** (`Cursor`-Vergleich) | Wer weiter kam, gewinnt |
| 1b | Fehler *mit* Cursor schlaegt Fehler *ohne* | nur wenn 1 nicht entscheidbar |
| 2 | **Fatalitaet** (`is_fatal`) | nur bei *gleicher* Stelle |
| 3 | **Prioritaet** | bei Gleichstand gewinnt der spaetere |

**Fortschritt kommt bewusst zuerst — auch vor einem `fail(..)`.** Wer mehr
Tokens erfolgreich verarbeitet hat, war naeher an der gemeinten Ableitung; ein
frueher stehendes `fail` beschreibt dann einen Zweig, den der Parser gar nicht
meinte. Belegt durch `error_abstraction_test::test_fail_vs_deep_error` (ein
tieferer Fehler schlaegt `fail`) gegen `:136` (bei *gleicher* Stelle gewinnt
`fail`).

Die Prioritaetsleiter (`error.rs`):

| Konstante | Wert | Wann |
|---|---|---|
| `PRIO_NORMAL` | 0 | gewoehnlicher Parsefehler |
| `PRIO_LABELED` | 10 | benannte Alternative (`# "…"`) an ihrer Grenze gescheitert |
| `PRIO_AGGREGATED` | 20 | zusammengefasste Erwartungen (`expected one of: …`) |
| `PRIO_STRUCTURAL` | 50 | `fail(..)` oder hinter einem Cut |

Nicht mehr Teil der Auswahl: **Stapeltiefe** und **Nachrichtenlaenge**. ADR-09
nennt die Laenge ausdruecklich als Instabilitaetsquelle; der Regelstapel dient
heute nur noch der Anzeige.

## Fatalitaet und Prioritaet sind getrennt

Frueher teilten sich beide einen Kanal (`priority = 50` bedeutete „fatal"). Das
war falsch: `fail(..)` soll hochprior sein, aber am Fortschrittsvergleich
teilnehmen. Fatal ist allein der **Cut** (`=>`). Deshalb gibt es das eigene Feld
`is_fatal`.

Der Cut legt die Ableitung fest: scheitert etwas hinter ihm, ist Zuruecksetzen
auf eine andere Alternative sinnlos, und der Fehler wird sofort durchgereicht.

## Der Kanal fuer verdeckte Fehler — `ParseContext::furthest`

Der wichtigste Mechanismus der heutigen Engine, und der am wenigsten
offensichtliche.

Ein rein funktionales Modell verliert jeden Fehler, den ein **erfolgreiches**
Zuruecksetzen ueberdeckt: ein `Ok` transportiert keinen Fehler. Beispiel —
`fn foo( 123 )` gegen `paren(separated(param, ",", min=0))`: das erste Element
scheitert an `123`, aber eine leere Liste ist gueltig, also liefert `separated`
ein `Ok` und wirft die aussagekraeftige Meldung weg. Uebrig bleibt eine
nichtssagende Meldung von weiter aussen.

Deshalb fuehrt `ParseContext` die **weiteste Fehlschlagstelle**:

- `record_failure(&err)` merged einen verworfenen Fehler in `furthest` — nach
  derselben Rangfolge wie `merge`.
- Aufgerufen an jeder Stelle, die einen Fehler verwirft: `parse_separated`
  (min=0-Pfad und Separator-Abbruch), `parse_repeated`, sowie
  `Optional`/`Repeat`/`Plus` und die Alternativenzweige im Codegenerator.
- `absorb(&other_ctx)` laesst den Mark aus einem verworfenen Kontext-Klon
  zurueckfliessen.
- `best_error(err)` waehlt am Ende den besseren aus zurueckgegebenem Fehler und
  Mark.

## Regelstapel — zwei Wege, die sich ergaenzen

Der Stapel wird **nur zur Anzeige** gefuehrt, nie zur Auswahl.

1. **Lebender Stapel im Kontext.** `enter_rule`/`exit_rule` umschliessen den
   Regelrumpf im generierten Code. `record_failure` haengt eine
   **Momentaufnahme** an den gemerkten Fehler. Nur so traegt ein *verdeckter*
   Fehler ueberhaupt Kontext.
2. **`push_rule` auf dem Rueckgabepfad.** Ein *zurueckgegebener* Fehler bekommt
   keine Momentaufnahme; die aeusseren Regeln haengen ihre Namen beim
   Herausreichen an.

Die Listen-Kombinatoren legen den Elementnamen ebenfalls auf den lebenden Stapel
(`"<item_label> <index>"`, `"separator"`) — daher `in function parameter 2` und
`in separator`.

## Rendern — einmal, ganz am Ende

Waehrend des Parsens wird `message` nie angefasst. Formatiert wird genau einmal
beim Uebergang nach `syn::Error` im Wrapper (`codegen/rule.rs`). Dort entsteht:

- die De-Snake-Case-Form (`deepest_err` → `deepest err`),
- die Kette `\nin X\nin Y` von innen nach aussen, dedupliziert,
- die Positionsangabe — nur wenn der Span echte Daten hat.

## Was der Grammatikautor davon sieht

| Werkzeug | Wirkung |
|---|---|
| `# "Label"` | ersetzt die Token-Ebene durch einen Klartextnamen; fliesst in `expected one of: …` |
| `item_label="…"` | benennt Listenelemente: `expected function argument`, `in function argument 2` |
| `fail("…")` | Meldung wortwoertlich, ohne `expected`-Praefix, hochprior |
| `=>` (Cut) | legt die Alternative fest; spaetere Alternativen werden nicht mehr probiert |
| `recover(rule, sync)` | ueberspringt bis zum Synchronisationstoken statt abzubrechen |

## Ein gescheitertes Listenelement

`parse_separated` behandelt einen gescheiterten Elementversuch nach einer Regel,
die in `ersetzt_meldung` (`core/grammar-kit/src/combinators.rs`) steht:

| Lage | Meldung | Rang |
|---|---|---|
| Element kam **voran** | seine eigene, samt Regelstapel | unveraendert |
| Element kam **nicht voran**, Meldung schon beschriftet | seine eigene (`expected \`x\`; found unexpected token \`y\``) | mind. `PRIO_LABELED` |
| Element kam **nicht voran**, Meldung unbeschriftet | `expected <item_label>` | mind. `PRIO_LABELED` |
| Am Ende der Eingabe bzw. der Gruppe | `<Ende>, expected <item_label>` | mind. `PRIO_LABELED` |

Die Regel gilt fuer den harten Pfad (`min > 0`, Fehler wird hochgereicht) und den
weichen (leere Liste erlaubt, Fehler nur gemerkt) gleich; der harte hebt
zusaetzlich auf `PRIO_STRUCTURAL`.

**Warum der Rang noetig ist.** Bei `fn f( 123 )` scheitert das Element an seiner
Anfangsstelle, die Liste ist optional, und direkt danach scheitert ein
optionales `","?` an *derselben* Stelle. Ohne den Rang einer Beschriftung
gewinnt der spaeter gemerkte Trenner-Fehler den Gleichstand, und aus
`expected function argument` wird das nichtssagende ``expected `,` ``.
`PRIO_LABELED` (10) schlaegt `PRIO_NORMAL` (0), also gewinnt das Element.

**Warum eine vorhandene Beschriftung stehenbleibt.** `finish_variants` erzeugt
`expected \`function parameter\`; found unexpected token \`123\`` — das nennt
zusaetzlich, was tatsaechlich dastand, und ist damit reicher als
`expected function parameter`. Am Ende der Gruppe gilt das Gegenteil: dort
waere eine Aufzaehlung irrefuehrend, weil gar nichts mehr kommt, und die
Angabe des Geltungsbereich-Endes ist die wichtigere (ADR 13, Punkt 3).

Belegt in `cxx-parser/tests/error_messages.rs::ungueltiges_argument_wird_als_fehlendes_element_gemeldet`
und `syn-grammar/tests/list_dx_test.rs`
(`beschriftetes_element_behaelt_seine_meldung_auch_bei_min1`,
`am_gruppenende_gewinnt_die_elementerwartung`).
