# Changelog

`grammar-kit` ist die Laufzeitbibliothek fuer von `syn-grammar` erzeugte Parser
und wird versionsgleich mit `syn-grammar` veroeffentlicht. Die vollstaendige
Liste der Aenderungen steht in
[`syn-grammar/CHANGELOG.md`](../../syn-grammar/CHANGELOG.md); hier stehen nur
die Punkte, die die API dieser Crate betreffen.

## [0.10.0]

### Breaking Changes

- **`ParseError<'a>` ersetzt `syn::Error` als Fehlertyp**, mit
  `ParseResult<'a, T> = Result<(T, Cursor<'a>), ParseError<'a>>` und den
  Prioritaetskonstanten `PRIO_NORMAL`/`PRIO_LABELED`/`PRIO_AGGREGATED`/`PRIO_STRUCTURAL`.
- **`ParseContext` heisst `ParseContext<'a>`.** Entfallen: `set_fatal`,
  `check_fatal`, `trigger_fail`, `record_error`, `take_best_error`,
  `is_best_error_deep`, `rule_stack()` sowie die Namensraum-Methoden direkt auf
  dem Kontext (jetzt unter `ctx.scopes`). Neu: `record_failure`, `absorb`,
  `best_error`, `furthest`, `enter_rule`/`exit_rule`, `enter_group`/`exit_group`,
  `end_of_scope_msg`. `record_span` gibt `syn::Result<()>` zurueck.
- **Alle bisherigen Kombinatoren sind entfernt** (`attempt`, `peek`,
  `not_check`, `attempt_recover`, `parse_ident`, `parse_int`, `skip_until`) und
  durch cursorbasierte ersetzt: `attempt_labeled`, `peek_syn`,
  `invoke_syn_parser`, `finish_variants`, `parse_separated`, `parse_repeated`.
- **`testing::TestResult<T, E>`** hat einen dritten Parameter `S = ()`; der
  `'static`-Bound auf `E` entfaellt; `assert_failure_contains` und
  `assert_failure_not_contains` liefern `Self`; `Testable` ist von
  `syn::Result<T>` auf `Result<T, E>` verallgemeinert.
- **Die Features `rt` und `trace` sind entfernt** - sie schalteten nichts.
  Ebenso das Makro `test_both_backends!` (auf nicht existierende Features
  gegated, wegen eines Dependency-Zyklus prinzipiell nicht lauffaehig) und das
  nie eingebundene Modul `transaction`.

### Added

- `SynParsable`: Marker mit `#[diagnostic::on_unimplemented]`, damit ein
  `syn::`-Typ ohne `Parse` eine verstaendliche Meldung erzeugt.
- `WithSpan` und das Ableitungsmakro `with_span` (aus `grammar-kit-macros`).
- `#![warn(missing_docs)]`; die oeffentliche API ist vollstaendig dokumentiert.
