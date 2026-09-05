# ADR 13: The Error Message Contract

## Status

Accepted. This is the binding catalogue of requirements from [`GOALS.md`](../../GOALS.md).

Relation to the existing ADRs: ADR-09, ADR-11 and ADR-12 describe **mechanics**
(structured error state, abstraction of error raising, aggregation). This ADR
describes the **observable result**. Where they disagree this ADR wins, because
it is backed by tests.

## Context

The requirements on error messages used to live implicitly in nine test files.
That made "enterprise level" unverifiable: there was no place stating what makes
a good message, and no way to decide whether a change is an improvement or a
regression.

All requirements below are derived from existing assertions and referenced by
location. What is not listed here is not a requirement.

Comparison is by **substring** (`core/grammar-kit/src/testing.rs:199-212`,
`assert_failure_contains`). Multi-line expectations are contiguous substrings
including `\n` — the order of the `in …` lines is therefore part of the
contract, the end of the message is not.

## Decision

### 1. Name the expectation

Every message says what was expected. Tokens in backticks, primitives without.

* `expected \`c\`` — `error_reporting_test.rs:65`
* `expected identifier` — `list_dx_test.rs:50`
* `expected \`,\`` — `list_dx_test.rs:82`

### 2. Labels replace the token level

An explicit label (`# "…"` on a variant, `item_label=` on lists) takes the place
of the internal token expectation.

* `expected \`type name\`` instead of `expected \`::\`` — `list_dx_test.rs:40`
* `expected one of: \`Letter A\`, \`Letter B\`` — `labeled_alternatives_test.rs:47`

### 3. Name what was found

* `; found unexpected token \`123\`` — `list_dx_test.rs:30`
* `unexpected end of input, …` — `list_dx_test.rs:72`, `trailing_comma_test.rs:41`
* `unexpected end of group, …` — `list_dx_test.rs:60`
* the same prefixes on an enumeration: `unexpected end of input, expected one of: …`,
  `unexpected end of group, expected one of: …` —
  `expectation_aggregation_test.rs::enumeration_names_the_end_of_scope`
* `unexpected match for rule \`bad\`; found \`bad\` in rule \`main\`` — `peek_not_test.rs:87`

### 4. Position

Format `at column N (line M)`, exactly once per message.

**Restriction, binding:** this is only printed when the span carries real
position data. Up to Rust 1.87 a procedural macro returned `(0,0)`; since 1.88
proc-macro2 sets `proc_macro_span_location` on stable too, and the project
requires that version (`rust-version = "1.88"`). The check stays regardless: a
span without position data is **omitted** rather than printed as `0` — e.g.
`Span::call_site()`, which still carries none.

The position serves **display** only. For **selecting** the best error it is
useless; point 8 applies there.

### 5. Rule stack

Multi-line, from inside out, deduplicated, rule names in space-separated form
(`deepest_err` → `deepest err`).

```
expected `c` at column 4 (line 1)
in deepest err
in main
```
— `error_reporting_test.rs:65`, likewise `:97` (`in inner rule`).

Nesting over several levels: `list_dx_test.rs:40`
(`in type name` → `in param` → `in function parameter 1` → `in signature`).

### 6. Aggregation of alternatives

If several alternatives fail at the same position, **one** message results:
`expected one of: …`, sorted and deduplicated, and it lists **every**
alternative that failed at its boundary — not only those whose first token
can be peeked. Each alternative contributes what it would have accepted:

* a literal its token text, a delimiter syn's word for it (`parentheses`,
  `square brackets`, `curly braces`);
* a built-in the expectation of its own error (`identifier`,
  `integer literal`, `string literal`);
* a called rule the enumeration it collected itself (union through nested
  rules);
* a labelled alternative its label, in place of all of the above.

An error carries this as `ParseError::expected`; the alternative chain unions
the sets (`finish_variants`). A single built-in keeps its own wording
(`expected integer literal`, no backticks) — the list form appears only when
there is something to enumerate.

* `expected one of: \`a\`, \`b\`` — `labeled_alternatives_test.rs:39`
* `expected one of: \`one\`, \`two\`, \`zero\`` (alphabetical) — `error_reporting_test.rs:81`
* also inside groups — `labeled_alternatives_test.rs:66`
* built-in next to delimiter: `expected one of: \`integer literal\`, \`parentheses\`` —
  `expectation_aggregation_test.rs::builtin_and_delimiter_are_both_listed`
* union through a called rule; a label replacing the inner list; a single
  built-in keeping syn's wording — `expectation_aggregation_test.rs`

### 7. Depth beats aggregation

If the parser got further in one alternative, that error displaces the
enumeration — `expected one of:` must then **not** appear.
— `labeled_alternatives_test.rs:57-58` (checks both, the positive and the negative case)

### 8. Selection order

Between competing errors, in this order:

1. **Progress** — whoever got further in the input wins.
   Measured on the cursor via `PartialOrd for Cursor` (syn 2.0.114,
   `src/buffer.rs:401-409`), **not** on line/column (see point 4)
2. **Fatality** — behind a cut (`=>`)
3. **Priority** — `fail` > label > default

**Progress comes before fatality and priority**, even before a `fail(..)`.
Whoever consumed more tokens successfully was closer to the intended
derivation; an earlier `fail` then describes a branch the parser did not mean.
Evidenced in `error_abstraction_test.rs:124` (`a b d` → `expected \`c\`` beats
the `fail` at column 2) against `:136` (`a d` → at the same position `hard fail`
wins).

It follows that **fatality and priority must be separate fields**: a cut fixes
the derivation and short-circuits the alternative chain; a `fail(..)` is merely
high-priority and must take part in the progress comparison. If both are
expressed through `priority`, `fail` short-circuits the chain and wrongly wins
against the deeper error.

*Earlier versions of this ADR listed fatality first. That contradicted the tests
and is corrected — the tests are the specification.*

The length of the message is **not** a criterion (ADR-09 names it as a source of
instability).

Evidenced in `error_abstraction_test.rs:124` (depth beats `fail` priority) and
`:136` (at the same position `fail` wins).

### 9. Cut

`=>` suppresses the errors of all later alternatives completely.
— `error_abstraction_test.rs:30,88,95`, `fail_test.rs:38`

### 10. `fail("msg")`

The text appears verbatim, without an `expected` prefix and without an
auto-label.

* `zero is not allowed` — `error_abstraction_test.rs:57`
* `hard fail` — `error_abstraction_test.rs:136`
* `foo cannot be followed by bar` — `fail_test.rs:38`

### 11. List diagnostics

* Item index in the stack: `in function parameter 2` (`list_dx_test.rs:50`),
  `in item 3` (`trailing_comma_test.rs:41`), `in function argument 3` (`list_test.rs:159-161`)
* Separator context: `in separator` — `list_dx_test.rs:82`
* Minimum count with actual value: `expected at least 2 items, found 1` — `list_test.rs:112`

### 12. Errors from action blocks

A `syn::Error` raised by the user in an action block is passed on unaltered and
only enriched with position and rule stack — not overwritten with `expected …`.

```
expected 'a' at column 4 (line 1)
in inner
in outer
```
— `error_reporting_test.rs:152`. **Not met today**; marked explicitly as the
target state in the test (`:149-151`).

### 13. Lazy formatting

The message is **never** modified during parsing. Rule names, labels and
position are assembled only at the transition to `syn::Error`. That keeps the
selection of point 8 independent of the textual form and makes results
deterministic.
— ADR-09

### 14. Evidence on the real macro path

At least one test must check the message through the **procedural macro path**,
not via `parse_str`. The proc-macro2 fallback that `parse_str` uses behaves
differently from a real macro in several places — what happens there is
otherwise never seen.

**Met** by the `trybuild` cases `runtime_error_real_macro`,
`runtime_ok_real_macro` and `joint_operator_real_macro` in `syn-grammar/tests/ui/`,
fed by the helper crate `syn-grammar/tests/ui-macro/`. Exactly this test showed
that the assumption previously recorded here — that no span inside a macro
carries a position — no longer holds since Rust 1.88.

## Consequences

* "Enterprise level" is measurable from here on: points 1-14 are met or not.
* Points 12 and 14 are open today and thus named gaps instead of invisible
  defects.
* Point 8 requires moving the selection from `span.start()` to the cursor.
  That is a behavioural change at the core and the reason the diagnostics are
  being rebuilt at all.
