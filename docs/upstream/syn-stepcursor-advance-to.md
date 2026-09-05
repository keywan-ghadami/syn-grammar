# Upstream request to `syn`: `StepCursor::advance_to`

**Status:** Draft, deliberately **shelved** (decision of 2026-09-01). The
benefit is demonstrably zero in observable behaviour (see "Evidence"), and syn
is conservative about API additions — a PR without pressure mostly consumes
other people's attention. The draft remains submittable should that change:
for instance if a primitive is needed that does not fail at its entry
position.
**Reference:** ADR 15, stage 4.

## What it is about

`syn::parse::advance_step_cursor` is `pub(crate)`:

```rust
pub(crate) fn advance_step_cursor<'c, 'a>(proof: StepCursor<'c, 'a>, to: Cursor<'c>) -> Cursor<'a>
```

The source of `ParseBuffer::step` itself says a public version would be safe
(syn 2.0.117, `src/parse.rs`):

> In some cases it may be necessary for `R` to contain a `Cursor<'a>`. Within
> Syn we solve this using `advance_step_cursor` which uses the existence of a
> `StepCursor<'c, 'a>` as proof that it is safe to cast from `Cursor<'c>` to
> `Cursor<'a>`. **If needed outside of Syn, it would be safe to expose that API
> as a method on `StepCursor`.**

The request is exactly that: expose the existing function as a method. No new
behaviour, no new invariant.

## The patch

Six lines in `impl<'c, 'a> StepCursor<'c, 'a>`:

```rust
/// Converts a cursor derived from this step cursor into one carrying the
/// lifetime of the underlying parse stream.
///
/// The existence of a `StepCursor<'c, 'a>` is proof that `'c` outlives `'a`;
/// see the comments on the struct definition.
pub fn advance_to(self, to: Cursor<'c>) -> Cursor<'a> {
    advance_step_cursor(self, to)
}
```

`StepCursor` is `Copy`, so `self` by value is unproblematic.

## The use case

A parser generator whose error type carries a `Cursor` — in our case for the
progress comparison between competing errors (O(1) pointer comparison in the
shared `TokenBuffer`, independent of span positions):

```rust
pub struct ParseError<'a> {
    pub span: Span,
    pub at: Option<Cursor<'a>>,   // <- this is what it hinges on
    pub message: String,
    // ...
}
```

The rule body runs on a `ParseBuffer<'a>`, but individual primitives run on
the `Cursor` — they are O(1) there and need no stream. To run such a primitive
on the stream and advance it by exactly its result, `step` is the only way.
Since the closure must work for **every** lifetime `'c`, a `ParseError<'c>`
cannot leave it.

Today the error is therefore carried across the barrier **without** its cursor
and re-attached outside at the entry position:

```rust
let mut saved: Option<(Span, String, u8, bool)> = None;
let result = input.step(|sc| match f(*sc) {
    Ok((value, after)) => Ok((value, after)),
    Err(e) => {
        saved = Some((e.span, e.message, e.priority, e.is_fatal));
        Err(syn::Error::new(e.span, "unreachable"))   // only to deny `step` the advance
    }
});
```

With the method the detour disappears:

```rust
let result = input.step(|sc| match f(*sc) {
    Ok((value, after)) => Ok((value, after)),
    Err(e) => {
        fehler = Some(e.mit_cursor(e.at.map(|c| sc.advance_to(c))));  // keeps its position
        Err(syn::Error::new(e.span, "unreachable"))
    }
});
```

## What it does *not* bring

For us today, **nothing in observable behaviour**. All affected primitives
report their error at the entry position anyway, so the reconstruction is
exact. The test suite is equally green with both versions.

The gain is that the detour disappears, along with the tacit condition that a
primitive may only fail at its entry position.

That belongs in the request: it would be dishonest to claim urgency here that
does not exist. The argument is that syn itself calls the change safe and it
costs six lines — not that it is burning here.

## Evidence

Built locally against syn 2.0.117 with exactly this patch, with `step`
(`core/grammar-kit/src/stream.rs`) converted to the form above: **153 tests
green / 0 red**, identical to the unpatched state. The patch and the conversion
were reverted afterwards; the repo still holds the version without the API.

## Proposed route

A pull request against `dtolnay/syn` with the patch above, referencing the
existing comment in `step` in the text. An issue would be the weaker route: the
change is smaller than its description.
