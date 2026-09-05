# Error Handling — How the Engine Works

This document explains **how** the engine selects and renders an error.

**What** a message must contain is specified in
[`adr/adr13-error-message-contract.md`](adr/adr13-error-message-contract.md).
The ADR is the binding catalogue; where the two disagree, the ADR wins, not
this document.

> Earlier versions of this document described an engine that no longer exists
> (position comparison via line/column, priorities 0/1/2, message length as
> tie-break, `!` as a fatal marker). The text was rewritten against
> `core/grammar-kit/src/error.rs` and `context.rs`.

## The central constraint

Up to Rust 1.87, `Span::start()` inside a real procedural macro returned
`LineColumn { line: 0, column: 0 }` for every span:

```rust
#[cfg(not(proc_macro_span_location))]
Span::Compiler(_) => LineColumn { line: 0, column: 0 },
```

**Since Rust 1.88 this no longer holds** — proc-macro2 sets this `cfg` on
stable too (`build.rs`: `rustc >= 88 && compile_probe_stable(..)`). The project
requires `rust-version = "1.88"`, so positions are available. Evidenced by
`syn-grammar/tests/ui/runtime_error_real_macro.stderr`, a snapshot from a real
macro.

**The separation of comparison and display stays anyway.** Not out of
necessity but because it is better: the cursor comparison is a pointer
comparison in O(1); a position comparison would be more expensive and hangs on
a compiler property. Behaviour that only becomes correct from a certain Rust
version on is no foundation for this project's quality criterion.

Consequence: **comparison and display are separate.**

- **Comparison** uses the `Cursor`. `syn::buffer::Cursor` implements
  `PartialOrd` as a pointer comparison within the shared `TokenBuffer` — O(1)
  and independent of toolchain and span availability.
- **Display** uses the `Span`. Inside a procedural macro rustc underlines it in
  the editor itself; the textual form `at column N (line M)` is only printed
  when the span carries real position data.

## The error type

`core/grammar-kit/src/error.rs`:

```rust
pub struct ParseError<'a> {
    pub span: Span,              // display
    pub at: Option<Cursor<'a>>,  // selection
    pub message: String,
    pub priority: u8,
    pub is_fatal: bool,
    pub rule_stack: Vec<String>,
}
```

`at` is `None` only where no cursor is at hand when the error is created — for
instance when adopting a foreign `syn::Error`. Such errors lose every progress
comparison against one with a cursor.

## Which error wins — `ParseError::merge`

In exactly this order:

| Rank | Criterion | Note |
|---|---|---|
| 1 | **Progress** (`Cursor` comparison) | whoever got further wins |
| 1b | an error *with* a cursor beats one *without* | only if 1 cannot decide |
| 2 | **Fatality** (`is_fatal`) | only at the *same* position |
| 3 | **Priority** | on a tie the later one wins |

**Progress deliberately comes first — even before a `fail(..)`.** Whoever
consumed more tokens successfully was closer to the intended derivation; an
earlier `fail` then describes a branch the parser did not mean at all.
Evidenced by `error_abstraction_test::test_fail_vs_deep_error` (a deeper error
beats `fail`) against `:136` (at the *same* position `fail` wins).

The priority ladder (`error.rs`):

| Constant | Value | When |
|---|---|---|
| `PRIO_NORMAL` | 0 | ordinary parse error |
| `PRIO_LABELED` | 10 | a labelled alternative (`# "…"`) failed at its boundary |
| `PRIO_AGGREGATED` | 20 | merged expectations (`expected one of: …`) |
| `PRIO_STRUCTURAL` | 50 | `fail(..)` or behind a cut |

No longer part of the selection: **stack depth** and **message length**.
ADR-09 explicitly names the length as a source of instability; the rule stack
today serves display only.

## Fatality and priority are separate

They used to share one channel (`priority = 50` meant "fatal"). That was
wrong: `fail(..)` should be high-priority but take part in the progress
comparison. Fatal is only the **cut** (`=>`). Hence the dedicated field
`is_fatal`.

The cut fixes the derivation: if something fails behind it, backtracking to
another alternative is pointless, and the error is passed through immediately.

## The channel for hidden errors — `ParseContext::furthest`

The most important mechanism of today's engine, and the least obvious.

A purely functional model loses every error that a **successful** backtrack
covers up: an `Ok` carries no error. Example — `fn foo( 123 )` against
`paren(separated(param, ",", min=0))`: the first item fails at `123`, but an
empty list is valid, so `separated` returns `Ok` and throws the informative
message away. What is left is a meaningless message from further out.

That is why `ParseContext` tracks the **furthest failure position**:

- `record_failure(&err)` merges a discarded error into `furthest` — by the same
  ranking as `merge`.
- Called at every place that discards an error: `parse_separated` (the min=0
  path and the separator break), `parse_repeated`, plus
  `Optional`/`Repeat`/`Plus` and the alternative branches in the code generator.
- `absorb(&other_ctx)` lets the mark flow back from a discarded context clone.
- `best_error(err)` picks the better of returned error and mark at the end.

## Rule stack — two paths that complement each other

The stack is kept **for display only**, never for selection.

1. **Live stack in the context.** `enter_rule`/`exit_rule` bracket the rule
   body in the generated code. `record_failure` attaches a **snapshot** to the
   remembered error. Only this way does a *hidden* error carry any context at
   all.
2. **`push_rule` on the return path.** A *returned* error gets no snapshot; the
   outer rules append their names as they pass it outward.

The list combinators also put the item name on the live stack
(`"<item_label> <index>"`, `"separator"`) — hence `in function parameter 2` and
`in separator`.

## Rendering — once, at the very end

During parsing, `message` is never touched. Formatting happens exactly once at
the transition to `syn::Error` in the wrapper (`codegen/rule.rs`). That is
where these arise:

- the de-snake-cased form (`deepest_err` → `deepest err`),
- the chain `\nin X\nin Y` from inside out, deduplicated,
- the position — only if the span carries real data.

## Expectation sets — how `expected one of:` is built

Besides its message an error carries `expected: Vec<String>` — what would
have been accepted at `at`, as display names (`identifier`, `parentheses`,
`a number`). The primitives fill it (`ParseError::expecting`,
`take_single`, `rt::group`, the token filters), `step` carries it across
syn's `step` barrier, and `finish_variants` in the alternative chain unions
the sets of all branches that failed at their boundary:

- a labelled branch contributes its label instead of its own set;
- an unlabelled branch contributes its error's set — a built-in's
  expectation, or the list a called rule collected one level down;
- a branch that was never tried because its peek failed contributes the label
  derived at macro time (`analysis::expectation_label`: the literal text or
  syn's word for the delimiter).

If the union says exactly what the best error already says, that error is
returned unchanged, which keeps `expected integer literal` in syn's wording.
Otherwise the enumeration is rendered, with `; found unexpected token …` and,
at the end of the scope, the `unexpected end of input, ` / `… of group, `
prefix. Before this, a branch starting with a built-in was invisible in the
list: `factor = i:i32 | paren(…)` on `*` reported ``expected `Paren` ``.

## What the grammar author sees

| Tool | Effect |
|---|---|
| `# "Label"` | replaces the token level with a plain-text name; flows into `expected one of: …` |
| `item_label="…"` | names list items: `expected function argument`, `in function argument 2` |
| `fail("…")` | message verbatim, no `expected` prefix, high priority |
| `=>` (cut) | fixes the alternative; later alternatives are not tried |
| `recover(rule, sync)` | skips to the synchronisation token instead of aborting |

## A failed list item

`parse_separated` handles a failed item attempt by a rule that lives in
`replaces_message` (`core/grammar-kit/src/combinators.rs`):

| Situation | Message | Rank |
|---|---|---|
| The item **made progress** | its own, including rule stack | unchanged |
| The item **made no progress**, message already labelled | its own (`expected \`x\`; found unexpected token \`y\``) | at least `PRIO_LABELED` |
| The item **made no progress**, message unlabelled | `expected <item_label>` | at least `PRIO_LABELED` |
| At the end of the input or the group | `<end>, expected <item_label>` | at least `PRIO_LABELED` |

The rule applies equally to the hard path (`min > 0`, error is propagated) and
the soft path (empty list allowed, error only remembered); the hard path
additionally raises to `PRIO_STRUCTURAL`.

**Why the rank is needed.** In `fn f( 123 )` the item fails at its start, the
list is optional, and right after that an optional `","?` fails at the *same*
position. Without the rank of a label, the later-remembered separator error
wins the tie and `expected function argument` turns into the meaningless
``expected `,` ``. `PRIO_LABELED` (10) beats `PRIO_NORMAL` (0), so the item
wins.

**Why an existing label stays.** `finish_variants` produces
`expected \`function parameter\`; found unexpected token \`123\`` — that
additionally names what was actually there, and is therefore richer than
`expected function parameter`. At the end of the group the opposite holds: an
enumeration would be misleading there, since nothing more is coming, and
naming the end of the scope is what matters (ADR 13, point 3).

Evidenced in `cxx-parser/tests/error_messages.rs::invalid_argument_is_reported_as_missing_item`
and `syn-grammar/tests/list_dx_test.rs`
(`labelled_item_keeps_its_message_even_with_min1`,
`at_group_end_item_expectation_wins`).
