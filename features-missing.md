# Missing Features for Intuitive Grammar Definitions

This document outlines features that would make `syn-grammar` significantly more intuitive and ergonomic, based on an analysis of the current codebase and comparison with other parser generators.

## 1. List Separator Syntax (High Priority)

**Context:**
Parsing lists of items (e.g., arguments `a, b, c` or statements `stmt; stmt;`) is a fundamental requirement.
Currently, users must use verbose recursive rules or generic helpers like `rule list<T>(item, sep)`.

**Missing Feature:**
Native infix operators for separated lists, providing concise and readable definitions for common patterns.

**Proposed Syntax:**
*   **`item ** ","`**: Matches `item` repeated **0 or more** times, strictly separated by `,`.
    *   **Behavior:** Strict. Trailing separators are **forbidden**.
    *   **Return Type:** `Vec<T>`.
*   **`item ++ ","`**: Matches `item` repeated **1 or more** times, strictly separated by `,`.
    *   **Behavior:** Strict. Trailing separators are **forbidden**.
    *   **Return Type:** `Vec<T>`.

**Why it's intuitive:**
It replaces verbose generic helper calls with a clear, mathematical notation (`**` for repetition, `++` for one-or-more) that directly expresses the intent.

**Example Usage:**
```rust
grammar! {
    grammar Json {
        // Matches: "[1, 2, 3]"
        // Fails:   "[1, 2, 3,]" (Trailing comma)
        rule array -> Vec<i32> = 
            "[" elements:i32 ** "," "]" -> { elements }
    }
}
```

## 2. Labeled Alternatives for Error Reporting (High Priority)

**Context:**
When a rule with multiple alternatives fails, the error message defaults to "expected <token>", derived automatically from the first token of the failing alternatives.
This is often unhelpful (e.g., "expected `(`" instead of "expected expression").

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

**Why it's intuitive:**
It allows library authors to guide users with meaningful error messages without writing custom `impl Parse` logic.

## 3. Implicit Token Literals / Token Aliases (Medium Priority)

**Context:**
Users must repeatedly use string literals for common tokens (e.g., `"+"`, `"fn"`), leading to "string soup" in grammars.

**Missing Feature:**
A standard prelude or alias system for common Rust tokens, or implicit support for single-char tokens.

**Proposed Syntax:**
*   **Implicit Tokens:** Support `char` literals (e.g., `'+'`) to denote single-token punctuation vs multi-token strings.
*   **Prelude:** Standard aliases like `PLUS`, `MINUS`, `DOT` available by default.

**Why it's intuitive:**
Reduces clutter in grammars heavily reliant on punctuation.

## 4. Action Block Ergonomics (Low Priority)

**Context:**
When using optional parsers (`?`) or recovery, the bound variable is an `Option<T>`. Action blocks often require boilerplate `match` or `unwrap_or` calls.

**Missing Feature:**
Syntactic sugar for default values within the binding itself.

**Proposed Syntax:**
*   **`x:rule? = 0`**: If `rule` is missing (returns `None`), bind `x` to `0` automatically in the action block.

**Why it's intuitive:**
Simplifies the Rust code in the action block, keeping the grammar declarative.

