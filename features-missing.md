# Missing Features for Intuitive Grammar Definitions

This document outlines features that would make `syn-grammar` significantly more intuitive and ergonomic, based on an analysis of the current codebase and comparison with other parser generators.

## 1. Resolve Parsing Ambiguity (High Priority)

**Context:**
The current parser grammar has potential ambiguities when distinguishing between a rule call with arguments and a rule followed by a grouped expression.
For example, does `ident ( ... )` mean "call rule `ident` with arguments `...`" or "match rule `ident` then match a group `( ... )`"?

**Missing Feature:**
A clear strategy or syntax to resolve this ambiguity, ensuring that the parser correctly interprets the user's intent.

**Proposed Solution:**
*   **Enforce Restrictions:** Disallow whitespace between the rule name and the opening parenthesis for function calls (e.g., `rule(arg)` is a call, `rule (arg)` is a sequence).
*   **Alternative Syntax:** Use a specific sigil for calls (e.g., `@rule(...)`) or groups (e.g., `group(...)`).

## 2. Syntax for Not (!) and Peek (&) Operators (High Priority)

**Context:**
Currently, `syn-grammar` supports `not(...)` and `peek(...)` functional syntax.
While functional, standard EBNF and PEG grammars often use `!` for negative lookahead and `&` for positive lookahead.

**Missing Feature:**
Shorthand prefix operators for lookaheads.

**Proposed Syntax:**
*   **`!rule`**: Equivalent to `not(rule)`. Succeeds if `rule` does *not* match. Does not consume input.
*   **`&rule`**: Equivalent to `peek(rule)`. Succeeds if `rule` matches. Does not consume input.

**Why it's intuitive:**
Matches standard PEG notation, reducing verbosity and making grammars easier to read for those familiar with parser theory.

## 3. Support for "Extern" or Imported Rules (High Priority)

**Context:**
Complex grammars often span multiple files or need to reuse rules from other crates.
While `use` statements are supported, there is no dedicated syntax to declare that a rule is "external" and should not be generated but rather expected to exist in the scope or be imported from a specific location.

**Missing Feature:**
Explicit support for declaring external rules or importing entire grammar modules.

**Proposed Syntax:**
*   **`extern rule name -> Type;`**: Declares a rule that is implemented elsewhere (e.g., manually implemented or in another module).
*   **`import grammar Path;`**: Imports all rules from another grammar definition.

## 4. Parametric List Rules (High Priority)

**Context:**
Parsing lists of items (e.g., arguments `a, b, c`) is a common requirement.
We previously considered infix operators like `**` and `++`, but these were rejected in favor of a more robust, generic approach.

**Missing Feature:**
Implementation of **Architecture Decision Record (ADR) 004**.

**Proposed Syntax (from ADR 004):**
*   **`separated(rule, ",")`**: Matches `rule` separated by `,`.
*   **`repeated(rule)`**: Matches `rule` repeated.
*   **`separated<Vec>(rule, ",")`**: explicit container type.

**Reference:**
See `docs/adr/adr04-list.md` for the full specification and rationale.

## 5. Labeled Alternatives for Error Reporting (Medium Priority)

**Context:**
When a rule with multiple alternatives fails, the error message defaults to "expected <token>", derived automatically from the first token of the failing alternatives.

**Missing Feature:**
The ability to explicitly label an alternative to provide a human-readable description in the generated error message.

**Proposed Syntax:**
Add a `# "label"` annotation to alternatives.

```rust
rule expr
    = atom
    | "(" expr ")" # "parenthesized expression"
    | "[" expr "]" # "bracketed expression"
```

## 6. Implicit Token Literals / Token Aliases (Medium Priority)

**Context:**
Users must repeatedly use string literals for common tokens (e.g., `"+"`, `"fn"`).

**Missing Feature:**
A standard prelude or alias system for common Rust tokens, or implicit support for single-char tokens.

**Proposed Syntax:**
*   **Implicit Tokens:** Support `char` literals (e.g., `'+'`) to denote single-token punctuation.
*   **Prelude:** Standard aliases like `PLUS`, `MINUS`, `DOT`.

## 7. Action Block Ergonomics (Low Priority)

**Context:**
When using optional parsers (`?`) or recovery, the bound variable is an `Option<T>`.

**Missing Feature:**
Syntactic sugar for default values within the binding itself.

**Proposed Syntax:**
*   **`x:rule? = 0`**: If `rule` is missing, bind `x` to `0`.
