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

Seit dem Umbau vom Mai 2026 parst das syn-Backend **funktional über Cursor** statt über
`ParseStream`:

```rust
pub type ParseResult<'a, T> = Result<(T, Cursor<'a>), ParseError>;   // error.rs:5
```

Der Cursor wird per Wert durchgereicht; Backtracking heißt, den Cursor von vor dem
Versuch weiterzubenutzen (`Cursor` ist `Copy`). Fehler sind Rückgabewerte und werden per
`ParseError::merge` (`error.rs:34-48`) kombiniert — es gibt **keinen** globalen
Fehlerzustand mehr.

| Datei | Z. | Inhalt |
|---|---|---|
| `error.rs` | 80 | `ParseError` (span, message, priority, rule_stack), `merge`, `Display` |
| `context.rs` | 72 | `ParseContext`: Scopes, Lexical-Mode, `last_span` — **ohne** Fehlerzustand |
| `combinators.rs` | 221 | `peek_syn`, `invoke_syn_parser`, `attempt_labeled`, `parse_separated`, `parse_repeated` |
| `testing.rs` | 314 | `Testable`/`TestResult`, `assert_failure_contains` (Substring-Vergleich, :199-212) |

`invoke_syn_parser` (`combinators.rs:18`) ist die Brücke zu syns `Parse`-Impls: sie baut
aus dem Cursor einen `TokenStream`, lässt syn parsen und rückt den Cursor um die
verbrauchte Tokenzahl vor. syn bietet **keinen** öffentlichen Weg von einem `Cursor` zu
einem `ParseStream`, deshalb ist dieser Umweg für echte syn-AST-Typen unvermeidbar.

## Bekannte Schwachstellen

Belegt, nicht vermutet:

1. ~~**Diagnostik greift im Produkteinsatz nicht.**~~ *Erledigt.* `merge` vergleicht
   `Cursor` per `PartialOrd` (O(1), `src/buffer.rs`) statt `span.start()`. Die
   ursprüngliche Begründung — `(0,0)` im Prozedurmakro — gilt seit Rust 1.88 ohnehin
   nicht mehr; das Projekt verlangt diese Version. Die Cursor-Metrik bleibt, weil sie
   billiger und toolchain-unabhängig ist. Siehe `GOALS.md`.

2. **Der Brückenaufruf für syn-AST-Typen bleibt O(n).** Einzeltoken laufen inzwischen
   über `take_single`/`take_fixed` in O(1); gemessen brachte das Faktor 6,2 an einer
   generierten cxx-Bridge. Was bleibt: `invoke_parser_fn` materialisiert für echte
   AST-Typen (`syn::Type` und Verwandte) den Reststrom bis zum Ende der umschließenden
   Delimiter-Gruppe, weil `ParseBuffer::new` `pub(crate)` ist. Bei einem AST-Typ je
   Listenelement ergibt das quadratisches Verhalten in der Länge dieser Liste. Wege
   heraus stehen in `docs/adr/adr15-linear-parsing.md`.

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
