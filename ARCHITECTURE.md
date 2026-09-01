# Architektur (Ist-Zustand)

Beschreibt, wie der Code am 2026-08-30 auf `logic-changes` tatsächlich aussieht — nicht,
wie er gedacht war. Ziele stehen in [`GOALS.md`](GOALS.md).

Alle Zeilenangaben sind gegen den Stand dieses Dokuments geprüft.

## Aufbau

Der Weg von der Grammatik zum Parser hat drei Stufen:

```
grammar! { … }                          Makro-Eingabe (TokenStream)
      │
      ▼
core/syn-grammar-model                  FRONTEND, backendunabhängig genutzt
      parser.rs      1262 Z.   TokenStream → syntaktischer AST
      model.rs        364 Z.   → semantisches Modell (ModelPattern, 19 Varianten, :61)
      validator.rs    527 Z.   Ambiguität, Shadowing, indirekte Linksrekursion
      analysis.rs    1153 Z.   Bindings, Nullable, Cut, Linksrekursion, Token-Auflösung
      │                        Einstieg: parse_grammar::<B: Backend> (syn_grammar_model.rs:30)
      ▼
syn-grammar/syn-grammar-macros          CODEGEN (syn-Backend)
      codegen/rule.rs      273 Z.   Regeln, Varianten, Linksrekursion
      codegen/pattern.rs   687 Z.   die 19 Muster
      monomorphize.rs      420 Z.   Generics zur Makro-Zeit auflösen
      backend.rs           262 Z.   BuiltIn-Katalog des syn-Backends
      │
      ▼
core/grammar-kit + syn-grammar/src      RUNTIME, wird vom erzeugten Code aufgerufen
```

Der erzeugte Code spricht die Runtime über den Alias `rt` an
(`syn-grammar/src/syn_grammar.rs:5-9`), der `grammar_kit::*`, `builtins` und
`token_filter` bündelt.

## Frontend: `core/syn-grammar-model`

Parst die DSL, überführt sie ins semantische Modell und validiert sie.
Einstieg `parse_grammar::<B: Backend>(TokenStream)` (`syn_grammar_model.rs:30-41`):
`syn::parse2` → `.into()` → `validator::validate::<B>`.

`ModelPattern` (`model.rs:61`) hat **19** Varianten: `Cut`, `Lit`, `RuleCall`, `Group`,
`Bracketed`, `Braced`, `Parenthesized`, `Optional`, `Repeat`, `Plus`, `SpanBinding`,
`Recover`, `Peek`, `Not`, `Until`, `Count`, `LexicalScope`, `SpacedScope`, `Fail`.

Der `Backend`-Trait (`model/backend.rs:13-16`) hat genau eine Methode,
`get_builtins() -> &'static [BuiltIn]`. Er steuert **ausschließlich die Validierung** der
Builtin-Namen — er sagt nichts über Codegen. Es gibt keine gemeinsame Codegen-Abstraktion.

**Zum Namen:** Das Crate hieß `syn-grammar-model`, weil es einmal von zwei Backends
benutzt wurde. Seit dem Auszug von `winnow-grammar` (31.08.2026) hat es nur noch einen
Nutzer, und der Name passt wieder. Backendunabhängig war es ohnehin nur in der
*Nutzung*, nicht in den *Typen*: das Modell trägt `syn::Path`, `syn::Lit`, `syn::Type`,
`syn::ItemUse` (`model.rs:14,20-23,28,65,69-70`), und `analysis::resolve_token_types` /
`analysis::get_simple_peek` erzeugen `Token![…]`- und `syn::token::*`-Typen. Genau
deshalb hat winnow das Crate beim Auszug geforkt statt es weiter zu beziehen — nur so
kann es dort in Richtung `syn`-Freiheit weiterentwickelt werden.

## Runtime: `core/grammar-kit`

Der Rumpf einer Regel arbeitet auf einem **Strom** (`ParseBuffer`), die
Blatt-Primitiven weiterhin auf dem **Cursor**:

```rust
pub type Strom<'a>            = syn::parse::ParseBuffer<'a>;          // stream.rs
pub type StreamResult<'a, T>  = Result<T, ParseError<'a>>;            // stream.rs
pub type ParseResult<'a, T>   = Result<(T, Cursor<'a>), ParseError<'a>>; // error.rs
```

Eine Regel heißt `fn parse_x_impl<'a>(input: &Strom<'a>, ctx: &mut ParseContext<'a>)
-> StreamResult<'a, T>`. Bewusst `&Strom<'a>` und **nicht** syns Alias
`ParseStream<'a> = &'a ParseBuffer<'a>`: der Alias setzt die Lebensdauer der Referenz
mit der der Tokens gleich, womit ein `input.fork()` `'a` auf den Stapelrahmen verkürzen
würde — Fehler aus einer Gabel könnten den Aufruf dann nicht mehr verlassen. Genau die
braucht die Fehlerauswahl.

Zurücksetzen läuft über `gabel` (`fork`) und `uebernehmen` (`advance_to`, laut syn O(1)):
ein Versuch läuft auf der Gabel, erst der Erfolg wird eingespielt. Fehler sind
Rückgabewerte und werden per `ParseError::merge` (`error.rs`) kombiniert — es gibt
**keinen** globalen Fehlerzustand.

| Datei | Inhalt |
|---|---|
| `error.rs` | `ParseError` (span, `at`-Cursor, message, priority, `is_fatal`, rule_stack), `merge`, `Display` |
| `context.rs` | `ParseContext`: Scopes, Lexical-Mode, `last_span`, `furthest` — **ohne** Fehlerzustand |
| `stream.rs` | `Strom`, `parse_syn`, `parse_mit`, `gabel`/`uebernehmen`, `gruppe`, `schritt`, `token_nehmen` |
| `combinators.rs` | `peek_syn`, `take_single`/`SingleToken`, `parse_separated`, `parse_repeated`, `finish_variants` |
| `testing.rs` | `Testable`/`TestResult`, `assert_failure_contains` (Substring-Vergleich) |

`parse_syn` (`stream.rs`) ist der Zugang zu syns `Parse`-Impls und schlicht ein
`input.parse::<T>()` — O(Länge des Typs). Bis August 2026 lief das über eine Brücke, die
den Reststrom materialisierte und `Parser::parse2` daraus einen neuen `TokenBuffer` bauen
ließ; das war O(Rest) je Aufruf und damit quadratisch in der Länge einer Liste. Siehe
`docs/adr/adr15-linear-parsing.md`.

Umgekehrt bleiben Einzeltoken auf dem Cursor: `schritt` lässt eine Cursor-Primitive in
einer `ParseBuffer::step`-Episode laufen und rückt den Strom um genau ihr Ergebnis vor.
`step` verlangt eine Closure, die für **jede** Lebensdauer funktioniert, weshalb ein
`ParseError<'c>` sie nicht verlassen kann; `schritt` trägt den Fehler ohne seinen Cursor
hindurch und hängt ihn draußen an die Eintrittsstelle. Das ist keine Näherung — diese
Primitiven melden ihren Fehler ohnehin dort.

In Delimiter-Gruppen steigt `gruppe` über `syn::__private::parse_{parens,braces,brackets}`
ab. `AnyDelimiter::parse_any_delimiter` geht nicht: seine Rückgabe ist auf die Lebensdauer
von `&self` verkürzt, womit kein Fehler aus der Gruppe nach außen tragen würde.

## Bekannte Schwachstellen

Belegt, nicht vermutet:

1. ~~**Diagnostik greift im Produkteinsatz nicht.**~~ *Erledigt.* `merge` vergleicht
   `Cursor` per `PartialOrd` (O(1), `src/buffer.rs`) statt `span.start()`. Die
   ursprüngliche Begründung — `(0,0)` im Prozedurmakro — gilt seit Rust 1.88 ohnehin
   nicht mehr; das Projekt verlangt diese Version. Die Cursor-Metrik bleibt, weil sie
   billiger und toolchain-unabhängig ist. Siehe `GOALS.md`.

2. ~~**Der Brückenaufruf für syn-AST-Typen bleibt O(n).**~~ *Erledigt* (ADR 15,
   Stufe 3). Der Rumpf läuft auf einem `ParseBuffer`, der genau einmal gebaut wird;
   ein `syn::Type` kostet `input.parse::<T>()`. Gemessen an einer generierten
   Argumentliste mit 2000 Einträgen: 1,17 s → 5,3 ms, und aus quadratisch wurde
   linear. Der Preis ist eine `Rc`-Allokation je Rücksetzpunkt statt eines
   Cursor-Copies.

3. **Fehlende Diagnostik-Bausteine.** `expected one of: …`, Label-Bubbling, Item-Index
   im Regel-Stack (`in item 3`) existierten vor dem Umbau und fehlen seither.

4. ~~**Toter Code.**~~ *Erledigt.* `transaction.rs` (147 Z., nie als Modul deklariert),
   `macros.rs` (`test_both_backends!`, auf nicht existierende Features gegated) und die
   leeren Features `rt`/`trace` sind entfernt. `test_both_backends!` war zusaetzlich
   prinzipiell nicht reparierbar: sein Rumpf braucht `syn-grammar` und `winnow-grammar`,
   die beide von `grammar-kit` abhaengen - ein Zyklus. Sein Doctest galt nur deshalb als
   gruen, weil das Makro zu nichts expandierte.

## `cxx-parser`

Abnahme-Benchmark auf dem syn-Backend (`cxx-parser/Cargo.toml:8`). 5 Regeln,
`src/cxx_parser.rs:37-79`. Der interessante Teil ist die Übergabe an syn für alles nach
`:` und `->` (`syn::Type`, `syn::ReturnType`, `syn::Generics`, `syn::Macro`) — genau die
Grenze, an der eine Fremd-DSL in echte Rust-Syntax übergeht.

## `winnow-grammar` — ausgezogen

War bis zum 31.08.2026 ein zweites Backend auf demselben Frontend. Es lebt jetzt unter
<https://github.com/keywan-ghadami/winnow-grammar> und ist vollständig unabhängig: keine
Referenz mehr auf `syn-grammar`, `syn-grammar-model` oder `grammar-kit`.

Beim Auszug aufgelöst: das Frontend wurde als `winnow-grammar-model` geforkt; aus
`grammar-kit` wanderten nur `WithSpan` (4 Zeilen) und `testing.rs` (341 Zeilen, ohne
`syn`-Bezug) mit, das Crate selbst wurde nicht geforkt. Der eigentliche Blocker steckte
nicht in den Manifesten, sondern im erzeugten Code: `codegen/variants.rs` schrieb
`::grammar_kit::WithSpan` als absoluten Crate-Pfad, womit jedes Nutzer-Crate
`grammar-kit` direkt brauchte.

Vier Abhängigkeiten waren tot (kein einziger Import): `syn` und `syn-grammar-model` in
`winnow-grammar`, `syn-grammar` und `grammar-kit` in `winnow-grammar-macros` — wobei
`syn-grammar` das komplette syn-Backend in jeden winnow-Build zog.

`docs/adr/adr14-shared-context-pattern.md` ist mitgezogen.

## Veraltete Dokumente

* `ARCHITEKTUR_MANIFEST.txt` — beschreibt `core/grammar-kit/src/lib.rs`; die Datei gibt es
  seit dem Umbau nicht mehr.
* `PROJECT_STRUCTURE.md` — spekulativ formuliert („likely contains"), nennt weder
  `grammar-kit` noch `syn-grammar-model`, verweist auf ein nicht existierendes `testresults.txt`.
* `EXTENDING.md` — beschreibt eine API, die es nicht gibt
  (`parse_grammar_with_builtins`, `Lit(LitStr)`, 6 statt 19 Muster); der Beispielcode
  würde nicht kompilieren.
