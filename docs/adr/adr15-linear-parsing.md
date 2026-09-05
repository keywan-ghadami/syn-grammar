# ADR 15: The Road to Linear Parsing

**Status:** Accepted. Stages 0, 1 and 3 are implemented; stage 1 was superseded
by stage 3. Stage 2 is dropped. Stage 4 is prepared and shelved. The ADR is
thereby complete.
**Date:** 2026-08-31, updated 2026-09-01

## Context

> This section describes the state **before** the decision. Since stage 3 the
> rule body runs on a `ParseBuffer`; the bridge no longer exists.

The generated parser worked on `syn::buffer::Cursor`. For real syn AST types
(`syn::Type`, `Generics`, `ReturnType`, `Macro`, `Block`, `Visibility`) there is
no way from a `Cursor` to a `ParseStream`, so `invoke_parser_fn`
(`core/grammar-kit/src/combinators.rs`) materialised the remaining stream on
every call and let `Parser::parse2` build a **new `TokenBuffer`** from it.

With one AST type per list item, that gave quadratic behaviour in the length of
that list.

### Measured

A generated argument list, two grammars identical except for the argument's
type — `t:syn::Type` (bridge) against `t:any_ident` (O(1)):

| n | with bridge | without bridge |
|---|---|---|
| 100 | 3.36 ms | 221 µs |
| 500 | 76.99 ms | 1.31 ms |
| 2000 | **1.40 s** | **5.32 ms** |

Twenty times the input costs **417×** the time with the bridge and **24×**
without. The bridge is therefore not *one* factor but the **only** remaining
source of quadratic behaviour — everything else is already linear.

Exponent from the data points: log(21.5)/log(5) = 1.91 and log(15.6)/log(4) =
1.98. Cleanly quadratic, as the model "n parses × O(n) buffer construction"
predicts.

### Where the time goes

Not in `cursor.token_stream()`. That only walks the **top-level** token trees
and clones groups via `Rc` in O(1) (proc-macro2, `RcVec::clone`).

But in `TokenBuffer::new2` inside `parse2` (`syn`, `buffer.rs`): a **recursive
depth-first walk over all nested tokens** that constructs an `Entry` per token
(two for groups, plus offset back-patching), collects them in a growing `Vec`
and finally copies them into a `Box<[Entry]>`. In a real procedural macro,
each group additionally costs a round trip through the rustc bridge
(`proc_macro2::wrapper`, `DeferredTokenStream::new`).

At n=2000 that is ~555 µs per argument for on average ~4000 remaining tokens,
so ~140 ns per token. Too much for pure `Rc::clone` — the time is in allocation
and `Entry` construction.

**Consequence: "just optimise the materialisation" achieves little.
`TokenBuffer::new2` has to go, or be bounded to the length of the type.**

## The constraint, checked exhaustively

There is **no** public way `Cursor → ParseStream`, and that is deliberate:

* `ParseBuffer` has no constructor at all. There is only the free function
  `pub(crate) fn new_parse_buffer` (`syn`, `parse.rs`).
* The reason is in the source and is about soundness: `ParseBuffer` holds a
  `Cell<Cursor<'static>>` with `PhantomData<Cursor<'a>>`; an API that accepts a
  `Cursor<'a>` and trusts its lifetime would be unsound.
* `discouraged::Speculative::advance_to` needs **two** existing buffers and
  creates none.
* `discouraged::AnyDelimiter::parse_any_delimiter` does call
  `new_parse_buffer` from outside — but only for the **contents of a delimiter
  group** at the current position, and it already needs a `&ParseBuffer`.
* `ParseStream::step` is a one-way street: `StepCursor` derefs to `Cursor`, but
  `advance_step_cursor` is `pub(crate)`. The way back fails on the invariance
  of `'c` in `StepCursor<'c, 'a>`.
* No syn feature flag (`parsing`, `full`, `derive`, `proc-macro`, …) unlocks
  any of this.

Notably, syn's own comment on `step` says it *would* be safe to offer
`advance_step_cursor` as a method on `StepCursor` — it just is not done. See
stage 4.

## Decision: a staged plan

### Stage 0 — `peek_syn` without allocation (**implemented**)

`peek_syn` built a token stream **and a complete `TokenBuffer`** per peek. A
window of n tokens does not help, because a single token can be an arbitrarily
large delimiter group — `cursor.token_tree()` returns `{ …1000 tokens… }` as
*one* tree.

Now pure pointer arithmetic via `<P::Token as syn::token::Token>::peek(cursor)`
— exactly what syn's own `ParseStream::peek` does. Zero allocations.

`Peek::Token` and `Token::peek` are `#[doc(hidden)]`, i.e. without a semver
promise; hence encapsulated in exactly one place.

Matters mostly in the recover sync scan, which sits in a loop.

### Stage 1 — bound at the delimiter group (**implemented, then superseded**)

A `syn::Block` **is** exactly one `{}` token tree. `cursor.group(Delimiter::Brace)`
yields the contents and the following cursor in O(1); only the contents are
materialised, i.e. exactly what is parsed. `take_braced_block` assembles the
`Block` from that — the same thing syn's `impl Parse for Block` does
(`braced!` + `Block::parse_within`), just without a ParseStream.

For `syn::Macro` the same applies in spirit: a macro invocation is
`path ! (…)`, and the path before it consists only of identifiers and `::` — it
cannot contain a group. `take_upto_group` therefore materialises up to **and
including the first group**. If none comes, it falls back to the previous
behaviour; it never gets more expensive.

**Measured**, n entries in a list:

| n | `syn::Block` before | after | `syn::Macro` before | after |
|---|---|---|---|---|
| 100 | 3.37 ms | 665 µs | 3.06 ms | 219 µs |
| 500 | 82.52 ms | 2.80 ms | 73.17 ms | 890 µs |
| 2000 | **1.06 s** | **11.46 ms** | **1.15 s** | **3.60 ms** |

Factor 92 and 319 at n=2000, and in both cases **quadratic → linear** (twenty
times the input: 315× and 376× before, 17× and 16× after).

The effect is larger than expected because not only the materialisation
disappears but also the `TokenBuffer` construction over the rest — and that is
what dominates.

**What stage 1 did not solve:** `syn::Type`, `Generics`, `ReturnType`,
`Visibility`. They have no group as a boundary. Exactly these appear in every
function argument of the cxx benchmark, and there the quadratic behaviour
remained.

**Postscript.** Stage 3 made `take_braced_block` and `take_upto_group` moot and
both are removed again: they bypassed the materialisation that no longer exists.
Nothing is lost — the measurement above pointed the way to stage 3, because it
showed that not the materialisation dominates but the `TokenBuffer`
construction.

### Stage 2 — angle-bracket window for `syn::Type` (**dropped**)

The code generator does not know the follow set, but the structure of types is
scannable. Delimiter groups are each **one** token tree and thus automatically
opaque; at depth 0, `<` counts up and `>` counts down — except when the `>` is
immediately preceded by a joint `-` (`->` in `Fn(A) -> B`).

`<<` and `>>` are two separate `Punct`s in proc-macro2, so they correctly count
±2. Lifetimes (`'a`) do not touch the counter. Comparison operators do not occur
at type level.

**Sound for `syn::Type`, `ReturnType`, `Generics` — not for `Expr`.** There `<`
and `>` are real operators; `Expr` would stay on the full bridge, `Block` is
covered by stage 1.

Delivers O(n), keeps the architecture intact, costs a carefully tested scanner.
**Not implemented and no longer needed** — stage 3 solves the same problem for
all types and without a scanner of our own.

### Stage 3 — ParseStream first (**implemented**)

The body works on a `ParseBuffer` instead of the `Cursor`; the leaf primitives
run in short `step` episodes. A `syn::Type` costs `input.parse::<T>()`, i.e.
O(length of the type) instead of O(rest). The `TokenBuffer` is built exactly
once.

**The signature is the pivot.** A rule is

```rust
fn parse_x_impl<'a>(input: &Strom<'a>, ctx: &mut ParseContext<'a>) -> StreamResult<'a, T>
```

with `Strom<'a> = ParseBuffer<'a>` — deliberately **not** syn's alias
`ParseStream<'a> = &'a ParseBuffer<'a>`. The alias equates the lifetime of the
reference with that of the tokens; but an `input.fork()` only lives to the end
of the stack frame, so `'a` would be shortened to that frame and a
`ParseError<'a>` from a fork could no longer leave the call. With `&Strom<'a>`
the reference lifetime stays free. This was checked in advance on a running
spike, not inferred.

**Error selection is unchanged.** `ParseBuffer::cursor()` is public and returns
`Cursor<'a>`; `fork()` only clones the cursor cell, the `TokenBuffer` stays the
same. So `same_buffer` holds and `PartialOrd` compares cursors from fork and
parent stream as before. Checked against the source and confirmed in the spike.

**Two syn APIs had to be encapsulated**, both `#[doc(hidden)]` and therefore
without a semver promise — like `Token::peek` in `peek_syn` before, hence in one
place each:

* `syn::__private::parse_{parens,braces,brackets}` for descending into a
  delimiter group. The macros `parenthesized!` & co. do not work: their error
  path is a bare `return Err(syn::Error)`. `AnyDelimiter::parse_any_delimiter`
  does not work either — its return value is shortened to the lifetime of
  `&self`, so no error from inside the group would carry outward.
* `ParseBuffer::step` in `schritt`, to run cursor primitives on the stream.
  Since `step` demands a closure for **every** lifetime, a `ParseError<'c>`
  cannot leave it; `schritt` carries it through without its cursor and
  re-attaches it outside at the entry position. These primitives report their
  error there anyway.

**Cost.** Where a cursor copy sufficed, a backtracking point now costs a
`fork()` allocation. Affected: every alternative, every `?`/`*`/`+`, `peek`,
`not`, `recover` and every list item. Another difference: after an error the
stream may have advanced — backtracking is no longer free but must go through a
fork. The code generator does so at every backtracking point.

**Measured**, the same test program before and after, two grammars differing
only in the argument type, `--release`:

| n | `syn::Type` before | after | `any_ident` before | after |
|---|---|---|---|---|
| 100 | 3.30 ms | 326 µs | 230 µs | 184 µs |
| 500 | 75.80 ms | 1.43 ms | 822 µs | 819 µs |
| 2000 | **1.174 s** | **5.33 ms** | 4.01 ms | 3.11 ms |

Factor 220 at n=2000. The shape matters more: twenty times the input used to
cost 356×, now 16× — cleanly linear. And `syn::Type` is now within 1.7× of
`any_ident`, which contains no AST type at all; the cost of the bridge is not
reduced but gone.

The allocation cost of backtracking is included in these numbers and
disappears in the noise — `any_ident` even got faster.

### Stage 4 — upstream (**prepared, shelved**)

`syn::parse::advance_step_cursor` is `pub(crate)`. The source of
`ParseBuffer::step` says itself that a public version as a method on
`StepCursor` would be safe.

**The rationale changed with stage 3.** Originally this read: "with it,
cursor-first would be possible without detours and this ADR would be moot".
That is outdated — cursor-first is no longer the goal. What remains is smaller
and more concrete: `schritt` has to carry the error of a cursor primitive
**without** its cursor across the `step` barrier, because the closure must work
for every lifetime `'c` and a `ParseError<'c>` cannot leave it. With
`StepCursor::advance_to` the cursor could be lifted from `'c` to `'a` and the
error would stay untouched.

**Measured by its benefit: small.** All current primitives report their error
at the entry position, so the reconstruction in `schritt` is exact. Built
locally against a patched syn 2.0.117 with `schritt` converted: 153 tests green,
identical to the unpatched state — **no observable behavioural difference**. The
gain is the vanished detour and the vanished tacit condition that a primitive
may only fail at its entry position.

The draft request is in
[`docs/upstream/syn-stepcursor-advance-to.md`](../upstream/syn-stepcursor-advance-to.md).
**Shelved** (decision of 2026-09-01): no observable benefit, and a PR without
pressure mostly consumes other people's attention. Submittable should a
primitive be needed that does not fail at its entry position.

## Recommendation

Stage 3 is implemented and removes the last quadratic source. The goal of this
ADR is thus reached: the parser is linear in the input length.

**Stage 4 is no longer a performance topic** but tidying at an interface — six
lines in syn, no observable difference here. The draft is ready and shelved.
This ADR is thereby complete.

The rebuild is the reversal of what happened in May 2026. The reasoning back
then — backtracking becomes trivial — was right and is preserved by the fork
strategy: the code generator backtracks at the same places as before, just via
`fork`/`advance_to` instead of a cursor copy.

## Consequences

* The measurement is part of the acceptance of every stage. Reproducible with
  two grammars that differ only in the argument type.
* The promise now reads: linear in the input length. No parse step
  materialises the remaining stream any more.
* The price is in `docs/LIMITATIONS.md`: every backtracking point costs an `Rc`
  allocation.
* `extern` rules change their signature — see the CHANGELOG.
