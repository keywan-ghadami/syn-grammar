# Changelog

All notable changes to this project will be documented in this file.

## [0.9.0] - Entwurf, nicht veroeffentlicht

> Diese Version ist **noch nicht auf crates.io**. Die letzte veroeffentlichte
> Fassung ist 0.8.0; alles hier Aufgefuehrte ist gegenueber 0.8.0 zu lesen.

Die Fehlerdiagnose wurde neu aufgebaut. Die Engine ging dabei zwischenzeitlich
vom `ParseStream`-Modell auf funktionales Cursor-Parsing und ist mit ADR 15,
Stufe 3 wieder auf einen Strom umgestellt - diesmal mit der neuen Diagnostik und
ohne die Materialisierungs-Bruecke. Netto gegenueber 0.8.0: derselbe Stromtyp,
neue Fehlerauswahl, lineare Laufzeit. Der Abschnitt fasst rund 280 Commits
zusammen, die zwischenzeitlich nicht im Changelog gelandet waren.

Fuer Endnutzer, die nur `grammar! { … }` schreiben und die generierte
`parse_X(ParseStream) -> syn::Result<T>` aufrufen, ist der Umstieg klein: die
Signatur dieses Wrappers ist unveraendert. Betroffen sind vor allem
handgeschriebene Parser, die in eine Grammatik eingehaengt werden, und alles,
was auf konkrete Fehlermeldungstexte prueft.

### Breaking Changes

- **Crate- und Dateilayout.** Das Repo ist ein virtueller Workspace: `syn-grammar`
  liegt unter `syn-grammar/`, `grammar-kit` und `syn-grammar-model` unter `core/`.
  Alle `lib.rs`/`mod.rs` sind nach ihrem Modul benannt (`grammar_kit.rs`,
  `syn_grammar.rs`, `model.rs`, `codegen.rs`) und ueber `[lib] path` eingebunden.
  - **Impact**: Nur fuer Beitragende relevant, nicht fuer Nutzer der Crates.

- **`ParseError<'a>` ersetzt `syn::Error` als interner Fehlertyp.** Neu:
  `ParseResult<'a, T> = Result<(T, Cursor<'a>), ParseError<'a>>` und die
  Prioritaetskonstanten `PRIO_NORMAL`/`PRIO_LABELED`/`PRIO_AGGREGATED`/`PRIO_STRUCTURAL`.
  - **Impact**: Der Typ traegt eine Lifetime. Wer ihn in eigenen Signaturen
    nennt, muss sie durchreichen.

- **`ParseContext` heisst jetzt `ParseContext<'a>`.** Entfallen: `set_fatal`,
  `check_fatal`, `trigger_fail`, `record_error`, `take_best_error`,
  `is_best_error_deep`, `define`/`is_defined`/`enter_scope`/`exit_scope` direkt
  auf dem Kontext, `rule_stack()`. Neu: `record_failure`, `absorb`, `best_error`,
  `furthest`, `enter_rule`/`exit_rule`, `enter_group`/`exit_group`,
  `end_of_scope_msg`, `mode_stack`, `group_depth`. `record_span` gibt jetzt
  `syn::Result<()>` zurueck.
  - **Migration**: Fatalitaet sitzt am Fehler (`ParseError::is_fatal`), nicht
    mehr am Kontext. Namensraeume liegen unter `ctx.scopes`.

- **Alle Kombinatoren aus `grammar-kit` sind ersatzlos entfernt**: `attempt`,
  `peek`, `not_check`, `attempt_recover`, `parse_ident`, `parse_int`,
  `skip_until`. Sie nahmen einen `ParseStream`.
  - **Migration**: Die Entsprechungen heissen
    `peek_syn`, `finish_variants`, `parse_separated`, `parse_repeated`, sowie
    das neue Modul `stream` (`Strom`, `StreamResult`, `parse_syn`, `parse_mit`,
    `gabel`, `uebernehmen`, `gruppe`, `schritt`, `token_nehmen`).

- **Alle `builtins::parse_*_impl` haben eine neue Signatur**: statt
  `<T: CommonBuiltins>(&mut T, &mut ParseContext) -> syn::Result<X>` jetzt
  `<'a>(&rt::Strom<'a>, &mut ParseContext<'a>) -> StreamResult<'a, X>`. Das Trait
  `CommonBuiltins` samt seiner `impl for ParseStream` ist geloescht.
  `token_filter::{alpha, alphanumeric, digit, hex_digit, oct_digit}` bleiben
  Cursor-Primitiven (`Cursor<'a> -> ParseResult<'a, X>`) und werden ueber
  `rt::schritt` auf dem Strom ausgefuehrt.

- **Die generierte `parse_X_impl` nimmt einen `&rt::Strom<'a>`** (also einen
  `&syn::parse::ParseBuffer<'a>`) und liefert `rt::StreamResult<'a, T>`, das
  heisst `Result<T, ParseError<'a>>` ohne Cursor im Erfolgsfall.
  - **Impact**: Das ist der Einhaengepunkt fuer handgeschriebene Parser
    (`extern`-Regeln); deren Signatur aendert sich entsprechend. Der oeffentliche
    Wrapper `parse_X(ParseStream) -> syn::Result<T>` ist unveraendert -
    **Endnutzer sind nicht betroffen**.
  - Bewusst `&ParseBuffer<'a>` und nicht syns Alias
    `ParseStream<'a> = &'a ParseBuffer<'a>`: der Alias wuerde `'a` beim Forken auf
    den Stapelrahmen verkuerzen, womit Fehler aus einer Gabel den Aufruf nicht
    mehr verlassen koennten.
  - **Zurueckgesetzt wird jetzt ueber `rt::gabel`/`rt::uebernehmen`**
    (`fork`/`advance_to`) statt ueber einen Cursor-Copy. Nach einem Fehler ist der
    Strom moeglicherweise vorgerueckt; wer zuruecksetzen will, muss auf einer Gabel
    arbeiten. Der Codegenerator tut das an jeder Ruecksetzstelle.

- **`syn_grammar::Identifier` und `syn_grammar::StringLiteral` als
  Wurzel-Re-Exports entfallen.**
  - **Migration**: `syn_grammar::types::Identifier` bzw. `::types::StringLiteral`.

- **Das Makro `include_grammar` ist geloescht.** Es gab seit 0.2.0 nur noch eine
  erklaerende `compile_error!`-Meldung; jetzt meldet der Compiler
  "cannot find macro". Grammatiken gehoeren inline in `grammar! { … }`.

- **`syn_grammar_model::parse_grammar_with_builtins` existiert nicht mehr.**
  - **Migration**: `parse_grammar::<B: Backend>`. `model` re-exportiert
    `backend::*`/`types::*` nicht mehr flach.

- **Datenmodell**: `GrammarDefinition` verliert `inherits` und gewinnt
  `extern_rules`/`imports`; `Rule` gewinnt `return_type_kind` und `is_lexical`;
  `RuleCall.rule_name` heisst `rule_path`; `params` ist `Vec<RuleParameter>`.

- **Grossgeschriebene Regelnamen sind automatisch lexikalisch.** Zwischen den
  Mustern einer solchen Regel ist kein Zwischenraum mehr erlaubt.
  - **Impact**: Eine bestehende Grammatik mit einer grossgeschriebenen Regel
    aendert *still* ihr Verhalten. Entweder umbenennen oder `spaced(...)` setzen.

- **`separated(..., trailing=false)`** bricht bei einem Trenner ohne
  Folgeelement nicht mehr hart ab, sondern setzt weich vor den Trenner zurueck.
  - **Impact**: Grammatiken der Form `paren(liste? ","?)` funktionieren dadurch
    neu; Grammatiken, die auf den harten Fehler gebaut haben, akzeptieren still.

- **`any_ident` akzeptiert jetzt Schluesselwoerter** (`Ident::parse_any`) und ist
  damit nicht mehr funktionsgleich mit `ident`.
  - **Impact**: Wer sich darauf verlassen hat, dass `any_ident` an einem
    Schluesselwort scheitert (etwa zur Abgrenzung in einer Alternativenkette),
    muss auf `ident` wechseln.

- **Die Fehlerauswahl vergleicht zuerst den Fortschritt**, erst danach
  Fatalitaet und Prioritaet.
  - **Impact**: Ein frueh stehendes `fail("…")` verliert gegen einen tiefer
    gekommenen Normalfehler - wer mehr Tokens verarbeitet hat, war naeher an der
    gemeinten Ableitung. Bei *gleicher* Stelle gewinnt `fail` weiterhin.
    (Eine fruehere Fassung dieses Entwurfs sagte "`fail` takes precedence" ohne
    Einschraenkung; das galt nie fuer den Fortschrittsvergleich.)

- **Das Format des Regelkontexts hat sich geaendert**: statt des Praefixes
  `Error in rule 'inner': …` jetzt Suffixzeilen `\nin inner rule`, mit
  Unterstrichen als Leerzeichen, plus ` at column N (line M)` sofern der Span
  Positionsdaten traegt.

- **Viele feste Meldungstexte haben sich geaendert**, u. a.
  `No matching rule variant found` -> ``expected one of: `a`, `b`; found unexpected token `x` ``,
  `expected at least N items` -> `expected at least N <item>s, found M`,
  `unexpected match` -> ``unexpected match for rule `X`; found `Y` in rule `Z` ``.
  - **Migration**: Der verbindliche Katalog ist neu
    `docs/adr/adr13-error-message-contract.md`.

- **`grammar-kit::testing::TestResult<T, E>` hat einen dritten Parameter
  `S = ()`**; der `'static`-Bound auf `E` entfaellt;
  `assert_failure_contains`/`assert_failure_not_contains` liefern `Self` statt
  `()`; `Testable` ist von `syn::Result<T>` auf `Result<T, E>` verallgemeinert.

- **`grammar-kit`: die Features `rt` und `trace` sind entfernt** - sie schalteten
  nichts. Ebenso das nie funktionsfaehige Makro `test_both_backends!` und das
  nie eingebundene Modul `transaction`.

### Added

- **Lineares Parsen (ADR 15).** Kein Parseschritt materialisiert mehr den
  Reststrom. Ein `syn::Type` kostet `input.parse::<T>()` statt eines neuen
  `TokenBuffer` ueber den gesamten Rest der umschliessenden Delimiter-Gruppe.
  Gemessen an einer Argumentliste mit 2000 Eintraegen: 1,174 s -> 5,33 ms, und
  aus quadratischem wurde lineares Verhalten (zwanzigfache Eingabe kostete
  vorher 356x, jetzt 16x). Einzeltoken laufen ueber `take_single` in O(1),
  `peek_syn` ohne jede Allokation.

- **Cursor-basierte Diagnose-Engine.** Fortschritt wird ueber
  `syn::buffer::Cursor` verglichen (`PartialOrd`, O(1)) statt ueber Zeile/Spalte.
  Grund: der Cursor-Vergleich ist ein Zeigervergleich in O(1) und haengt an
  keiner Compilerversion. (Bis Rust 1.87 lieferte `Span::start()` im
  Prozedurmakro zudem fuer *jeden* Span `(0,0)`; seit 1.88 nicht mehr.)
- **Hochwasserstand fuer verdeckte Fehler** (`ParseContext::furthest`). Ein
  Fehler, den ein *erfolgreiches* Zuruecksetzen ueberdeckt (`?`, `*`,
  `separated(min=0)`), ueberlebt und wird gemeldet, wenn sonst nur eine
  nichtssagende Meldung uebrig bliebe.
- **Lebender Regelstapel** (`enter_rule`/`exit_rule`) mit Momentaufnahme, damit
  auch ein gemerkter Fehler seinen Regelkontext traegt.
- **`item_label=`** fuer `separated`/`repeated`: benennt Listenelemente und
  zaehlt sie (`expected function argument … in function argument 2`).
- **Trennung von Fatalitaet und Prioritaet**: `ParseError::is_fatal` fuer den
  Cut, `priority` fuer `fail(..)` und Labels.
- **Unterscheidung `unexpected end of group` / `unexpected end of input`** ueber
  die Gruppentiefe im Kontext.
- **`extern`-Regeln und `import`** in der Grammatik-DSL.
- **`lex(...)` und `spaced(...)`** zur expliziten Steuerung der
  Whitespace-Empfindlichkeit, plus die Grossschreibungs-Konvention.
- **`count(pattern)`**, das die Anzahl der Treffer liefert.
- **Vereinfachte Regelsyntax**: ohne `rule`-Schluesselwort, ohne Rueckgabetyp,
  ohne Aktionsblock.
- **Neue Builtins**: `any_ident`, `named_field`, `unnamed_field`, `visibility`,
  `generics`, `return_type`, `statements`, sowie `pat`, `inner_attrs` und
  `lit_byte`. `pat` schliesst die groesste Luecke: `syn::Pat` hat kein
  `impl Parse` und war ueber den `syn::`-Pfad nicht erreichbar.
- **`with_span`-Ableitungsmakro** und der `WithSpan`-Trait.
- **`DEBUG_GRAMMAR`**: Umgebungsvariable, die den generierten Code auf stderr
  ausgibt.
- **`cxx-parser`** als Abnahme-Benchmark (eigene Crate, Version 0.1.0).

### Fixed

- **`digit`, `hex_digit`, `oct_digit`** waren im Builtin-Katalog als
  `syn::Ident` deklariert, liefern aber `syn::LitInt`. Der deklarierte Typ
  steuert die Generics-Inferenz - eine generische Regel, die mit `digit`
  instanziiert wurde, erzeugte einen Compilerfehler im *generierten* Code.
- **Die "Undefined rule"-Pruefung** wurde von *jedem* `use`-Statement
  abgeschaltet. Sie haengt jetzt am Glob-Import (`use …::*;`), der als einziger
  unbekannte Regelnamen mitbringen kann. Ein Tippfehler im Regelnamen wird damit
  wieder sauber gemeldet statt als Folgefehler im generierten Code.
- **Verworfene Alternativ-Fehler gingen verloren**, wenn eine spaetere
  Alternative erfolgreich war und Eingabe uebrig liess. Statt einer Meldung wie
  `expected integer literal … in term … in expression` erschien nur syns
  `unexpected token`.
- **Ein `syn::`-Typ ohne `Parse`** (z. B. `syn::Field`) erzeugte einen rohen
  Trait-Bound-Fehler auf generiertem Code. Jetzt kommt eine Meldung auf der
  Zeile des Nutzers, die das passende Builtin nennt.
- **`peek_syn` und die Bruecken-Kombinatoren** scheiterten, sobald auf den
  Aufruf noch Tokens folgten - `Parser::parse2` verlangt, dass der gesamte
  Stream verbraucht wird. Die Bruecke ist mit ADR 15, Stufe 3 ganz entfallen.
- **Zero-Progress-Schutz** in den Wiederholungsschleifen: `*`, `+` und die
  Listen-Kombinatoren konnten bei einem Muster, das nichts verbraucht, endlos
  laufen.
- **`LexicalScope`/`SpacedScope`** raeumten bei einem Fehler den `mode_stack`
  nicht ab (`?` stand vor `exit_mode()`).
- **Der CI-Doc-Schritt** war rot: `syn::parse::<Token>()` ohne Backticks in
  einem Doc-Kommentar liess rustdoc unter `-D warnings` abbrechen.

### Documentation

- `docs/ERROR_HANDLING.md` neu geschrieben - es beschrieb die alte Engine
  (Positionsvergleich, Prioritaeten 0/1/2, Nachrichtenlaenge als Tiebreak) und
  eine Label-Syntax, die es nie gab.
- `docs/adr/adr13-error-message-contract.md` neu: der verbindliche Katalog fuer
  Fehlermeldungen, jeder Punkt mit Testfundstelle.
- `GOALS.md` und `ARCHITECTURE.md` neu; `EXTENDING.md` und `docs/adr/adr1.md`
  als ueberholt bzw. zurueckgezogen markiert.
- `SYNTAX.md`: der Label-Operator `#`, `item_label`, die `spanned_*`-Familie und
  die Span-Bindung `@` waren nicht dokumentiert.
- `#![warn(missing_docs)]` in `grammar-kit` und `syn-grammar`; 58 fehlende
  Doc-Kommentare ergaenzt.

### Aeltere Eintraege dieses Entwurfs

Der 0.9.0-Entwurf war schon vor dem oben beschriebenen Umbau begonnen worden.
Diese Eintraege stammen aus der ersten Haelfte und gelten weiterhin - sofern
oben nichts anderes steht.

#### Added
- **Consolidated Error Messages with Labeled Alternatives**: The parser can now produce a single, clear error message when multiple alternatives fail at the same position (e.g., "expected one of: an expression, a statement"). This is enabled by a new labeling mechanism, which uses rule names as default labels and supports explicit labels via the `# "label"` syntax. This replaces ambiguous, single-alternative errors with a helpful summary of all valid possibilities.
- **High-Priority Manual Error Reporting**: Added the `fail("message")` built-in rule, which always fails with a custom error message. Errors generated by `fail` carry a high priority and win against other errors **at the same position**, allowing grammar authors to provide precise, context-aware messages instead of the parser's default "expected" text. They do not, however, override an error that got *further* into the input - see the error-selection entry above.
- **Implicit Token Literals and Aliases**: The grammar parser now supports `char` literals (e.g., `'+'`) as a shorthand for single-token string literals. It also includes a prelude of common token aliases (e.g., `PLUS` for `"+"`).
- **Parametric List Rules (ADR 004)**: Added `separated` and `repeated` built-in rules for concise list parsing.
    - `separated(rule, sep, min=0, trailing=false)`: Parses a list of items separated by a delimiter.
    - `repeated(rule, min=0)`: Parses a list of items without a separator.
    - Supports custom container types via generics (e.g., `separated<HashSet>(...)`), defaulting to `Vec`.
- **Named Arguments**: Added support for named arguments in rule calls (e.g., `rule(key=value)`), used by the new list rules.
- **Generic Arguments in Rules**: Added support for generic type arguments in rule calls (e.g., `rule<T>(...)`), enabling the container specification for list rules.
- **Until**: Added support for the `until` pattern (e.g., `body:until(";")`), which consumes tokens until a terminator pattern is found. The terminator is not consumed. This is useful for parsing unstructured content or content with a known delimiter.
- **Group Bindings**: Added support for binding a group of patterns to a variable (e.g. `var:("a" | "b")`). This captures the result of the group (which matches the inner pattern).

#### Fixed
- **Built-in Rule Arguments**: Fixed an issue where the validator incorrectly forbade arguments for built-in rules. This allows backend-specific built-ins (or future portable built-ins) to accept arguments as needed.

#### Breaking Changes
- **Removed `<_>` Disambiguation**: Removed the `<_>` hack for rule arguments and generic calls. Instead, rule calls with arguments now require named arguments (e.g., `rule(name=arg)`) or template parameters to disambiguate them from EBNF groupings. Built-ins like `fail`, `peek`, `not` as well as `separated` and `repeated` are unaffected and can be used without named arguments.

## [0.8.0]

### Added
- **Literal Bindings**: Added support for binding string literals directly to variables in grammar rules (e.g., `label:"literal"`). This resolves to the `syn::Token` corresponding to the literal.
- **Optional Literal Bindings**: Added support for optional literal bindings (e.g., `label:"literal"?`, which resolve to `Option<Token>`.
- **Span Binding on Literals**: Extended the span binding syntax (`@`) to support string literals (e.g., `"literal" @ span_var`), allowing direct capture of a literal's span.

### Breaking Changes
- **Backend API**: The internal data model and macro infrastructure were updated to support the new literal binding features. This constitutes a breaking change for downstream backend implementers (e.g., `winnow-grammar`) who must now handle these new syntax-tree nodes. This change is not breaking for end-users who only write grammars.

## [0.7.1]

### Fixed
- **Typed Parameter Validation**: Fixed a bug where rule parameters with explicit types (e.g., `rule list<T>(item: Type)`) were incorrectly flagged as "Undefined rule" by the validator. The validator now correctly recognizes all parameters, regardless of whether they have explicit types or not.

## [0.7.0]

### Added
- **Portable Primitives**: Introduced a distinction between `PORTABLE_BUILTINS` (`ident`, `integer`, `alpha`, etc.) and `SYN_SPEC_BUILTINS` (`rust_type`, `lit_str`, etc.). This clarifies the portability contract for authors of alternative backends (e.g., `winnow-grammar`), encouraging a rich, shared vocabulary of common parsing concepts.
- **Portable Types**: Introduced backend-agnostic wrapper types `Identifier`, `StringLiteral`, and `SpannedValue<T>`. These types implement `ToTokens`, allowing them to be used seamlessly in `quote! { ... }` macros while provides a consistent API across different backends.
- **Numeric Built-ins**: Added a comprehensive set of portable numeric built-ins:
    - **Signed Integers**: `i8`, `i16`, `i32`, `i64`, `i128`, `isize` (and `int*` aliases).
    - **Unsigned Integers**: `u8`, `u16`, `u32`, `u64`, `u128`, `usize` (and `uint*` aliases).
    - **Floating Point**: `f32`, `f64`.
    - **Alternative Bases**: `hex_literal`, `oct_literal`, `bin_literal` (parses into `u64`).
- **Spanned Primitives**: Added `spanned_` variants for all primitives (e.g., `spanned_i32` returns `SpannedValue<i32>`), allowing easy access to source location data.
- **`whitespace` Primitive**: Added the `whitespace` assertion, which ensures a gap (non-adjacency) between two tokens.
- **Lookahead Operators**: Added support for positive (`peek(...)`) and negative (`not(...)`) lookahead operators.
    - `peek(pattern)`: Succeeds if the pattern matches, but does not consume input.
    - `not(pattern)`: Succeeds if the pattern does *not* match. Does not consume input.
- **`alpha` Primitive**: Added the `alpha` built-in primitive, which matches an identifier composed entirely of alphabetic characters.
- **Architecture**: Introduced `Backend` trait and `CommonBuiltins` to decouple the grammar definition from the `syn` implementation, paving the way for other backends.
- **ADR for Primitives**: Added an Architecture Decision Record (`docs/adr/adr1.md`) to document the design for handling character-level, byte-level, and token-level primitives across different backends.
- **ADR for Portable Types**: Added an Architecture Decision Record (`docs/adr/adr2.md`) to document the design for portable types and explicit backend contracts.
- **Higher-Order Generic Rules**: Added support for generic rules with untyped grammar parameters (patterns) and generic type parameters (e.g., `rule list<T>(item) -> Vec<T>`).
- **Monomorphization**: Implemented compile-time monomorphization of generic rules, allowing the creation of reusable grammar patterns without runtime overhead.
- **Generic Arguments in Rules**: Generic parameters now support standard Rust trait bounds, which are enforced on the inferred types.
- **Numeric Argument Support**: Updated the parser to support numeric literals as arguments to rules (e.g., `value(10)`), enabling more flexible rule parameterization.
- **ADR for Generic Rules**: Added an Architecture Decision Record (`docs/adr/adr3.md`) documenting the design of higher-order generic rules and macro-time monomorphization.
- **Restored Tests**: Added back `test_rule_arguments` and `test_multiple_arguments` to ensure rule parameter functionality works as expected.
- **Shadowing Detection**: Compile-time validation now reports errors when a grammar rule alternative is shadowed by a previous prefix alternative or is an exact duplicate.

### Changed
- **Backend-Agnostic Model**: The `syn-grammar-model` crate now exposes `parse_grammar_with_builtins`. This allows backend authors to validate grammars against their own set of built-in rules.
- **Backend Author Guide**: `EXTENDING.md` has been rewritten to focus on how to build custom parser generator backends using `syn-grammar` as the frontend DSL.
- **Validation Errors**: Shadowing and duplicate alternatives are now treated as hard errors instead of warnings to ensure grammar correctness.

### Fixed
- **Repetition Syntax**: Fixed a regression where repetition patterns were incorrectly requiring brackets `[...]` instead of parentheses `(...)`.
- **Linter Warnings**: Resolved multiple `clippy` warnings (unused variables, collapsible if-blocks, approximate constants).
- **Float Testing**: Improved float primitive tests to use proper epsilon comparison for accuracy.

### Breaking Changes
- **Portable Types for Primitives**: To improve backend portability (see ADR 2), several built-in parsers now return backend-agnostic types instead of `syn`-specific ones.
    - `ident` now returns `syn_grammar::Identifier` instead of `syn::Ident`.
    - `string` now returns `syn_grammar::StringLiteral` instead of `String`.
    - **Impact**: Action blocks that expect the previous `syn` types must be updated to use the new portable types (e.g., use `name.text` instead of `name.to_string()` or rely on `Display` impl).
- **Renamed `Spanned<T>` to `SpannedValue<T>`**: The `Spanned<T>` struct has been renamed to `SpannedValue<T>` to avoid name collisions with the `syn::spanned::Spanned` trait.
    - **Impact**: Code that uses `Spanned<T>` (e.g. return types of `spanned_*` built-ins) must be updated to use `SpannedValue<T>`.
- **Built-in Rule Resolution**: The precedence of built-in rules (like `ident`, `string`) has changed. They are no longer hardcoded keywords but are now provided as default implementations in `syn_grammar::builtins`.
    - **Impact**: If you define a rule named `ident` in your grammar, it will now *shadow* the built-in `ident` parser instead of being ignored. This fixes a long-standing limitation but may change behavior if you accidentally relied on the shadowing being ignored.

## [0.6.0]

### Added
- **`use super::*`**: The generated parser module now includes `use super::*;` by default, allowing parsers to seamlessly access other items defined in the parent module.
- **Use Statement Support**: Added support for standard Rust `use` statements within the grammar block (e.g., `use syn::Ident;`). These are passed through to the generated parser module.

## [0.5.0]

### Added
- **Span Binding Syntax**: Added support for the `name:parser @ span_var` syntax. This allows binding the result of a parser to `name` and its span to `span_var` simultaneously (e.g., `id:ident @ span`).

### Deprecated
- **Spanned Literal Parsers**: The `spanned_*_lit` built-in parsers (e.g., `spanned_int_lit`, `spanned_string_lit`) are deprecated. Use the standard literal parsers with the new span binding syntax instead (e.g., `lit_int @ span`).

## [0.4.0]

### Added
- **Token Recognition in Literals**: Enhanced parsing of string literals in the grammar to support multi-token sequences and complex combinations (e.g. `"?."`, `"@detached"`).
- **Pretty Error Printing**: The testing framework now pretty-prints `syn::Error` with source code context and underlining when assertions fail.
- **Outer Attributes**: Added support for parsing outer attributes (`#[...]`) via the `outer_attrs` built-in.
- **Span Binding**: Added support for capturing spans via `rule @ span_var` syntax.

### Improved
- **Error Spans**: Generated code now uses specific token spans instead of `Span::call_site()` where possible, resulting in more precise error highlighting in IDEs.

### Fixed
- **Documentation**: Fixed failing doctests in README, cleaned up examples, and clarified usage of brackets and delimiters.

### Internal
- **Testing**: Stabilized testing infrastructure.

## [0.3.0]

### Breaking Changes
- **Runtime Dependency**: Generated parsers now depend on the new `grammar-kit` crate (formerly `syn-kit`). Users must add `grammar-kit = "0.3.0"` to their `Cargo.toml`.
- **Renamed Built-in Parsers**:
  - `int_lit` has been renamed to **`integer`** (returns `i32`).
  - `string_lit` has been renamed to **`string`** (returns `String`).
  - This change distinguishes high-level value parsers from the low-level token parsers (`lit_int`, `lit_str`).

### Added
- **Attributes on Rules**: Rules can now be decorated with attributes, such as doc comments (`///`) or `#[cfg(...)]`.
- **Error Recovery**: Added `recover(rule, sync_token)` to handle syntax errors gracefully by skipping tokens until a synchronization point.
- **Rule Arguments**: Rules can now accept parameters (e.g., `rule value(arg: i32) -> ...`), allowing context to be passed down the parser chain.
- **Grammar Inheritance**: Grammars can inherit from other modules (e.g., `grammar MyGrammar : BaseGrammar`), enabling the use of external or manually written "custom parsers".
- **Testing Utilities**: Added `syn_grammar::testing` module with fluent assertions (`assert_success_is`, `assert_failure_contains`) to simplify writing tests for grammars.
- **Improved Error Reporting**: The parser now prioritizes "deep" errors (errors that occur after consuming tokens) over shallow errors.
- **New Built-in Parsers**:
  - `lit_int` -> `syn::LitInt`
  - `lit_char` -> `syn::LitChar`
  - `lit_bool` -> `syn::LitBool`
  - `lit_float` -> `syn::LitFloat`
  - `spanned_int_lit` -> `(i32, Span)`
  - `spanned_string_lit` -> `(String, Span)`
  - `spanned_float_lit` -> `(f64, Span)`
  - `spanned_bool_lit` -> `(bool, Span)`
  - `spanned_char_lit` -> `(char, Span)`

### Internal
- **Architecture**: Extracted runtime utilities (backtracking, error reporting, testing) into a separate `grammar-kit` crate.

## [0.2.0]

### Removed
- **`include_grammar!`**: Support for external grammar files (`.g`) has been removed.
  - **Reason**: Error reporting within external files was poor, making debugging difficult.
  - **Migration**: Please move your grammar definitions inline using the `grammar! { ... }` macro to benefit from full Rust compiler diagnostics and IDE support.

### Fixed
- **Generated Code**: Fixed usage of `syn` macros (`bracketed!`, `braced!`, `parenthesized!`) by removing incorrect error propagation (`?`).
- **Generated Code**: Changed rule variant generation to use a flat list of checks instead of `else if` chains, ensuring correct "first match wins" behavior and error fallthrough.

### Internal
- **Architecture**: Extracted grammar parsing, validation, and analysis into a separate `syn-grammar-model` crate. This enables the creation of alternative backends (e.g., `winnow`) in the future.
