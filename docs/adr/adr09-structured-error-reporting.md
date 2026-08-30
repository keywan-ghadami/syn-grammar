# ADR: Enhanced Error Reporting with Structured Context and Lazy Formatting

## Status

Accepted — umgesetzt, dann verloren, wird neu gebaut.

Der hier beschriebene `ErrorState` existierte bis Commit `0aace8a` in
`core/grammar-kit/src/lib.rs` und wurde beim Umbau auf Cursor-Parsing (Mai 2026)
ersatzlos gelöscht. „Proposed" war dadurch irreführend.

**Eine Festlegung dieses ADR ist überholt:** Punkt 2 der Auswahlhierarchie („Location
(Progress)") stützt sich auf `start_span` und damit auf `Span::start()`. Auf stable Rust
liefert das im Prozedurmakro immer `(0,0)`
(`proc-macro2-1.0.106/src/wrapper.rs:449-450`), die Heuristik ist dort also wirkungslos.
Ersatz ist `PartialOrd for Cursor` (syn 2.0.114, `src/buffer.rs:401-409`).
Siehe [ADR 13](adr13-error-message-contract.md), Punkt 8.

Das beobachtbare Ergebnis regelt [ADR 13](adr13-error-message-contract.md); dieses
Dokument beschreibt die Mechanik dahinter.

## Context
The current error reporting in `syn-grammar` struggles with precision and consistency, particularly in complex grammars involving labeled alternatives, deep nesting, and manual failure points (`fail` built-in).

**Problems:**
1.  **Stringly-Typed Errors**: Context (like "in rule `A`") is baked into the error string immediately. This makes it impossible to inspect the error structure later (e.g., to de-duplicate, prioritize, or format differently).
2.  **Ambiguous "Best" Error**: The `ParseContext` relies on heuristics like "deepest error wins," but conflates depth with string length when depths are equal, leading to unstable error messages.
3.  **Loss of Semantic Intent**: When a user provides a label (e.g., `# "Expression"`), this high-level intent is often lost or buried under low-level token errors if the parser dives deep into a sub-rule and fails there.
4.  **Priority Conflicts**: Errors triggered by `fail` (high priority) versus parsing errors (natural priority) are hard to balance.

We need "Enterprise Level" error messages: clean, precise, context-aware, and respectful of developer intent (labels).

## Decision
We will refactor the error handling system to use **Structured Error State** with **Lazy Formatting**.

### 1. Structured Error State
Instead of storing a simple `syn::Error` and a few flags, `ParseContext` will store a rich `ErrorState` struct:

```rust
#[derive(Clone)]
pub struct ErrorState {
    /// The underlying raw error (e.g., "expected `+`")
    pub err: syn::Error,
    
    /// The stack of rules active when the error occurred.
    /// Ordered from root -> leaf.
    pub rule_stack: Vec<String>,
    
    /// The specific span where the error occurred.
    pub start_span: Span,
    
    /// User-defined priority.
    /// 0 = Standard parse error
    /// 1 = Labeled alternative failure (intermediate) - *New Concept*
    /// 2 = Explicit `fail` / `cut` (highest)
    pub priority: u8,
    
    /// If true, this error represents a "cut" failure or fatal error 
    /// that should stop backtracking.
    pub is_fatal: bool,
    
    /// Optional: A high-level label overriding the message.
    /// e.g. "expected Expression" derived from a labeled alternative.
    pub label: Option<String>,
}
```

### 2. Intelligent Error Selection
When recording an error, we compare the new candidate against the existing `best_error` using a strict hierarchy:

1.  **Fatality**: If the new error is fatal, it wins immediately (and stops parsing).
2.  **Location (Progress)**: The error that occurred furthest in the input stream (deepest `start_span`) generally wins. This assumes that matching more tokens implies being "closer" to the correct parse.
3.  **Priority**: If locations are equal, higher priority wins.
    *   Explicit `fail` > Labeled Failure > Standard Error.
4.  **Context specificity**: If locations and priorities are equal, we prefer the error with a *deeper* rule stack (more specific) or one that carries a user-defined label.

### 3. Lazy Formatting & Display
The `syn::Error` message string is **never mutated** during the parsing process. We only prepend "in rule `X`" or apply labels when `take_best_error()` is called at the very end.

*   **Labels**: If a `label` is present in the `ErrorState`, the final message might be formatted as: `"expected {label}"` (possibly appending details: `"expected {label}: {raw_error}"` if verbose mode is on).
*   **Rule Traces**: The rule stack is formatted cleanly. We can detect recursion or excessive nesting and simplify the output (e.g., `main > expr > term` instead of `in rule main: in rule expr: ...`).

### 4. Handling Labeled Alternatives (`# "Label"`)
When a rule variant has a label, the generated code currently just adds a comment or metadata. We will enhance this:
*   If a labeled variant fails *without* consuming input (or making significant progress), we treat it as a failure to match that *concept*.
*   We can introduce a mechanism to "bubble up" labels. If a deep error occurs but it's not "far enough" (e.g. just started matching), the outer labeled alternative might override the error with "expected {Label}".

## Implementation Details

### `ParseContext` Changes
*   `record_error` will now take `label: Option<String>` and `priority: u8`.
*   Remove string manipulation logic from `record_error`.
*   Implement `format_error` inside `take_best_error`.

### `grammar-kit` Macro Changes
*   The code generation for labeled alternatives needs to pass the label to `ctx` when a failure occurs.
*   The code generation for `fail` built-in needs to pass high priority.

## Consequences

### Positive
*   **Clarity**: Users see "expected Expression" instead of "expected `(`" if that's what the developer intended.
*   **Debugging**: The full rule stack is preserved, allowing for detailed debug traces if needed.
*   **Stability**: Removing string length heuristics makes tests deterministic.

### Negative
*   **Complexity**: The `ParseContext` logic becomes more sophisticated.
*   **Migration**: Existing tests expecting exact string matches (including the "in rule `X`" prefixes) will need updates.

## Plan
1.  Update `ParseContext` and `ErrorState` in `grammar-kit`.
2.  Implement the comparison logic.
3.  Implement the lazy formatting.
4.  Update the generated code (macros) to utilize the new `label` capability (this might be a separate task, but the runtime support must be there).
5.  Fix failing tests.
