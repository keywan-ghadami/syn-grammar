# ADR 11: High-Level Error Handling Abstractions in ParseContext

## Status
Proposed

## Context
The current implementation of the parser generator relies heavily on low-level manipulation of the `ParseContext` within the generated code (via macros). The generated code directly accesses methods like `take_best_error`, `trigger_fail`, `suppress_label`, and manually manages the `is_fatal` flag.

This creates a "Leaky Abstraction" where the code generation logic needs to understand the intricate details of the error handling state machine. It increases the complexity of the macros (`syn-grammar-macros`) and makes the runtime library (`grammar-kit`) harder to evolve without breaking generated code.

Specifically, the handling of:
- The "Cut" operator (`=>`) manually sets `ctx.is_fatal = true`.
- The `fail` keyword manually triggers `ctx.trigger_fail()`, takes the best error, and returns a new error.
- Sequence validation (e.g., trailing separators) is often implemented inline in the macros.

## Decision
We will introduce high-level abstraction methods in `ParseContext` to encapsulate these common parsing patterns. The generated code will call these semantic methods instead of manipulating the state directly.

### New Abstractions

1.  **`commit()`**
    *   **Purpose**: To signal that the parser has passed a point of no return (e.g., after a Cut `=>`).
    *   **Behavior**: Sets the internal `is_fatal` flag to `true`.
    *   **Usage**: Called by the generated code immediately after the left-hand side of a Cut succeeds.

2.  **`raise_failure(msg, span)`**
    *   **Purpose**: To report a deliberate semantic or structural failure that should override other parsing attempts.
    *   **Behavior**: 
        *   Clears any existing "best error" (to prevent shadowing by less relevant previous errors).
        *   Sets the error priority to High (Level 2).
        *   Suppresses automatic labeling (to avoid "expected Identifier" when reporting "integer too large").
        *   Returns an `Err` with the provided message.
    *   **Usage**: Used by the `fail` keyword and potentially by built-in validation rules.

3.  **`check_trailing_separator()`** (Future Consideration)
    *   **Purpose**: To validate list structures.
    *   **Behavior**: Checks if a separator exists but is followed by nothing (if trailing is disallowed) or other specific list constraints.

## Consequences

### Positive
*   **Reduced Macro Complexity**: The code generation logic becomes simpler and more readable. instead of emitting 5 lines of state manipulation, it emits one method call.
*   **Encapsulation**: The internal logic of *how* a failure is prioritized or how a cut is implemented stays within `grammar-kit`. We can change the implementation (e.g., add new priority levels) without recompiling all grammars (mostly).
*   **Consistency**: All generated parsers will behave identically regarding these mechanisms.

### Negative
*   **Migration Effort**: We need to update `grammar-kit` first, and then update `syn-grammar-macros` to use the new methods.
*   **Dependency**: `syn-grammar-macros` will require a minimum version of `grammar-kit`.

## Implementation Plan
1.  Add `commit()` and `raise_failure()` to `ParseContext` in `grammar-kit`.
2.  Add unit tests in `grammar-kit` to verify their behavior interacts correctly with `best_error` and `is_fatal`.
3.  (Later) Update macros to use these new methods.
