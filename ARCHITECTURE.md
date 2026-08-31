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

**Zum Namen:** Das Crate heißt `syn-grammar-model`, wird aber auch von `winnow-grammar`
benutzt. Backendunabhängig ist es allerdings nur in der *Nutzung*, nicht in den *Typen*:
das Modell trägt `syn::Path`, `syn::Lit`, `syn::Type`, `syn::ItemUse`
(`model.rs:14,20-23,28,65,69-70`), und `analysis::resolve_token_types` /
`analysis::get_simple_peek` erzeugen `Token![…]`- und `syn::token::*`-Typen — das ist
syn-Codegen im gemeinsamen Frontend, den winnow nie aufruft.

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

1. **Diagnostik greift im Produkteinsatz nicht.** `merge` (`error.rs:38-46`) und die
   Anzeige (`error.rs:61-62`) vergleichen `span.start().line/.column`. Auf stable Rust
   sind das im Prozedurmakro immer `(0,0)`
   (`proc-macro2-1.0.106/src/wrapper.rs:449-450`). Siehe `GOALS.md`. Ersatzmetrik:
   `PartialOrd for Cursor` (syn 2.0.114, `src/buffer.rs:401-409`), O(1).

2. **`invoke_syn_parser` ist O(n²).** Sie ruft pro Token `cursor.token_stream()` auf,
   materialisiert also den gesamten Reststrom, klont und zählt ihn. Für Einzeltoken ist
   das vermeidbar — `Cursor` hat O(1)-Zugriffe (`ident()`, `punct()`, `literal()`,
   `group()`).

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

## `winnow-grammar`

Zweites Backend auf demselben Frontend, erzeugt winnow-Kombinatoren über
`Stateful<LocatingSlice<&str>, ParseContext<S>>`. Fehler sind `winnow::error::ContextError`;
die Diagnostik-Anforderungen aus `GOALS.md` erfüllt es nicht und verspricht es auch nicht.

**Es wird ein eigenes Projekt** (siehe `GOALS.md`, Nicht-Ziele). Für den Auszug relevant:
Seine syn-Kopplung zur Laufzeit ist weitgehend zufällig — `syn` in
`winnow-grammar/Cargo.toml:19` und `syn-grammar-model` in `:16` werden in `src/` nirgends
benutzt, `winnow-grammar-macros/Cargo.toml:13` zieht `syn-grammar`, ohne es je zu
referenzieren (dadurch baut der winnow-Build heute das komplette syn-Backend mit).
Aus `grammar-kit` braucht es nur `WithSpan` und `testing`. Nach diesen Schnitten bliebe
zur Laufzeit `winnow` + `lasso`.

## Veraltete Dokumente

* `ARCHITEKTUR_MANIFEST.txt` — beschreibt `core/grammar-kit/src/lib.rs`; die Datei gibt es
  seit dem Umbau nicht mehr.
* `PROJECT_STRUCTURE.md` — spekulativ formuliert („likely contains"), nennt weder
  `grammar-kit` noch `syn-grammar-model`, verweist auf ein nicht existierendes `testresults.txt`.
* `EXTENDING.md` — beschreibt eine API, die es nicht gibt
  (`parse_grammar_with_builtins`, `Lit(LitStr)`, 6 statt 19 Muster); der Beispielcode
  würde nicht kompilieren.
