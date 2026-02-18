# Architecture Decision Record (ADR) 004: Separated List Syntax and Trailing Separator Strategy

**Date:** 2026-05-20
**Status:** Proposed
**Context:**
Parsing lists of items separated by a token (e.g., arguments `a, b, c` or statements `stmt; stmt;`) is a fundamental requirement for almost every grammar.

Currently, `syn-grammar` users have two options, both suboptimal for simple cases:
1.  **Manual Recursion:** Writing `list = item | item "," list` is error-prone and verbose.
2.  **Generic Helper:** Using `rule list<T>(item, sep)` works but is syntactically heavy.

We need a dedicated syntax that is:
1.  **Intuitive:** Familiar to users of other parser generators or EBNF variants.
2.  **Precise:** Clearly distinguishes between "strict separation" (e.g., JSON arrays) and "trailing allowed" (e.g., Rust structs).
3.  **Type-Safe:** Maps cleanly to Rust types (`Vec<T>` vs `syn::punctuated::Punctuated<T, P>`).

## 1. Survey of Standards and Tools

We analyzed how other standards and tools handle separated lists and trailing separators.

### 1.1. Standards (EBNF/ABNF)
Most formal standards **do not** have a specific operator for separated lists, relying on repetition and groups.

*   **ISO EBNF (ISO/IEC 14977):**
    *   Syntax: `item, { separator, item }`
    *   *Note:* The `{ ... }` syntax means 0 or more.
    *   *Trailing:* Must be explicit: `item, { separator, item }, [ separator ]`.
*   **W3C EBNF (XML):**
    *   No specific operator. Uses recursive definitions.
*   **ABNF (RFC 5234):**
    *   Syntax: `1*element` (repetition).
    *   *Separators:* Implied by the element definition or explicit: `elem *(SEP elem)`.

### 1.2. Parser Generators (Lexer/Parser)
*   **ANTLR 4:**
    *   No built-in operator.
    *   Pattern: `expr (',' expr)*`.
    *   *Trailing:* `expr (',' expr)* ','?`.
*   **Tree-sitter (JavaScript/C):**
    *   `seq(rule, repeat(seq(',', rule)))`.
*   **PEG.js:**
    *   Manual tail recursion or mapping.

### 1.3. Rust-Specific Tools
Rust tools often provide specialized support because Rust macros heavily use separated lists.

*   **`macro_rules!` (Rust Built-in):**
    *   Syntax: `$($x:expr),*` (Strict).
    *   Syntax: `$($x:expr),+` (Strict, 1 or more).
    *   *Trailing:* `$($x:expr),* $(,)?` (The `$(,)?` pattern explicitly allows a trailing comma).
*   **Nom (Combinator Library):**
    *   `separated_list0(sep, item)` (Strict).
    *   `separated_list1(sep, item)` (Strict).
    *   *Trailing:* handled via `terminated` or custom combinators.
*   **Chumsky:**
    *   `.separated_by(sep)` (Strict).
    *   `.allow_trailing()` (Method chain to allow trailing).
*   **Pest (PEG):**
    *   `item ~ (sep ~ item)*` (Strict).
    *   `item ~ (sep ~ item)* ~ sep?` (Trailing).

## 2. Analysis of the Problem Space

There is a conflict between **Syntactic Conciseness** and **Semantic Precision**.

1.  **The "Vec" Case (Data):** Users parsing data (JSON, config files) usually want a `Vec<T>`. They don't care about the separator tokens.
2.  **The "Punctuated" Case (Syntax):** Users parsing Rust-like syntax (macros, DSLs) often need `syn::punctuated::Punctuated<T, P>`. They **must** preserve separator tokens for accurate source spans and re-printing.
3.  **Trailing Separators:**
    *   In `Vec` cases, trailing separators are often syntax errors (JSON).
    *   In `Punctuated` cases, trailing separators are often idiomatic (Rust).

## 3. Decision

We will adopt a **Two-Tiered Approach**:
1.  **Infix Operators (`**`, `++`)** for strict, `Vec`-producing lists.
2.  **Generic Rules (`punctuated`)** for rich, `Punctuated`-producing lists with trailing support.

This decision avoids creating a cryptic "mini-language" for trailing commas inside the operator syntax (e.g., `** ,?`).

### 3.1. Strict Separation Syntax (`**`, `++`)
These operators map to the "Strict/Vec" use case.

*   **`rule ** sep`**: Matches `rule` repeated **0 or more** times, strictly separated by `sep`.
    *   **Grammar:** `rule (sep rule)*`
    *   **Return Type:** `Vec<T>` (where `rule` returns `T`).
    *   **Trailing:** **Forbidden**.
*   **`rule ++ sep`**: Matches `rule` repeated **1 or more** times, strictly separated by `sep`.
    *   **Return Type:** `Vec<T>`.
    *   **Trailing:** **Forbidden**.

#### Example: Strict JSON Array

```rust
// Grammar Definition
grammar! {
    grammar Json {
        rule array -> Vec<i32> = 
            "[" elements:i32 ** "," "]" -> { elements }
    }
}
```

*   **Input:** `[1, 2, 3]` -> **Match**
*   **Input:** `[1, 2, 3,]` -> **Error** (Expected `]`, found `,`)
*   **Action Block Context:** `elements` is available as `Vec<i32>`.

### 3.2. Rich Separation (The `punctuated` Generic)
For cases requiring optional trailing separators or token preservation, we use the `punctuated` generic rule. This aligns with `syn`'s philosophy.

*   **Syntax:** `punctuated(rule, sep)`
*   **Return Type:** `syn::punctuated::Punctuated<T, P>`
*   **Trailing:** **Allowed**.

#### Example: Rust Struct Fields

```rust
// Grammar Definition
grammar! {
    grammar Struct {
        // Defines a list that allows trailing commas
        rule fields -> Punctuated<Field, Token![,]> = 
            p:punctuated(field, ",") -> { p }
            
        rule field -> Field = ...
    }
}
```

*   **Input:** `a: i32, b: i32` -> **Match**
*   **Input:** `a: i32, b: i32,` -> **Match** (Trailing allowed)

### 3.3. Why not `rule ** sep,`?
We considered syntax like `rule ** sep,` (noting the comma) to imply "allow trailing".
**Rejection Reason:** It is visually subtle and hard to read. Explicitly using `punctuated(...)` signals the intent to handle punctuation richer than just a delimiter.

## 4. Consequences

*   **Clarity:** Users know `**` produces a `Vec` and enforces strictness.
*   **Interoperability:** Users needing `syn`'s specific data structures have a clear path (`punctuated`).
*   **Future Proofing:** We leave open the possibility of adding a `.allow_trailing()` modifier in the future if the community demands it, without breaking the core `**` syntax.
