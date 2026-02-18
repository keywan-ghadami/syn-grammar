# Architecture Decision Record (ADR) 004: Parametric List Rules via Built-in Generics

**Date:** 2026-05-20
**Status:** Accepted
**Context:**
Parsing lists of items separated by a token (e.g., arguments `a, b, c` or statements `stmt; stmt;`) is a fundamental requirement for almost every grammar.

Currently, `syn-grammar` users have two suboptimal options:
1.  **Manual Recursion:** Writing `list = item | item "," list` is error-prone, verbose, and hard to maintain.
2.  **Ad-hoc Helpers:** Using `rule list<T>(item, sep)` lacks a standardized, optimized implementation in the backend.

We initially considered introducing infix operators (`**` for strict lists, `++` for non-empty lists) to solve this. However, after a deep analysis of the "strangeness budget," extensibility constraints, and type coupling, we realized that operators are a dead end for a robust system programming parser generator.

We need a dedicated syntax that is:
1.  **Intuitive:** Self-explanatory to users without requiring a lookup table for obscure operators.
2.  **Precise:** Clearly distinguishes between "strict separation" (e.g., JSON) and "trailing allowed" (e.g., Rust structs).
3.  **Type-Agile:** Must support `Vec<T>`, `HashSet<T>`, `Punctuated<T, P>`, or custom collections without forcing unnecessary intermediate allocations.

## 1. Decision

We **reject** the introduction of new infix operators (`**`, `++`).

Instead, we will introduce **Built-in Parametric Rules** into the standard grammar scope. These rules mimic Rust's function and generic syntax (`<T>`) to provide a familiar, extensible interface for list parsing.

The core rules are:
* `separated(...)`: For separated lists (e.g., `a, b, c`).
* `repeated(...)`: For non-separated sequences (e.g., `a a a`).

### 1.1. The `separated` Interface

The signature of the rule is defined as:

```rust
separated<Container = Vec>(
    rule: Rule, 
    separator: Token, 
    min: usize = 0, 
    trailing: bool = false
) -> Container<Rule::Output>
```

### 1.2. Usage Examples

#### Scenario A: The Standard List (JSON Array)
Equivalent to the rejected `**` operator, but readable. Defaults to `Vec`.

```rust
rule array -> Vec<Val> = "[" items:separated(value, ",") "]"
```

#### Scenario B: Custom Collection Types (The HashSet Case)
Users can specify the target collection type via generics. This prevents the "Vec-Monopoly" where users are forced to allocate a `Vec` only to convert it later. This is crucial for performance-critical parsers.

```rust
// Parses directly into a HashSet, assuming the underlying parser 
// engine (winnow) supports FromIterator/Extend.
rule unique_ids -> HashSet<Ident> = 
    keys:separated<HashSet>(ident, ",")
```

#### Scenario C: Configuration (Trailing Separators & Non-Empty)
Using named arguments allows us to extend functionality indefinitely without breaking syntax changes.

```rust
// Rust-like struct fields: 
// 1. Must have at least one field (min = 1)
// 2. Allows trailing comma (trailing = true)
// 3. Preserves tokens (Punctuated)
rule fields -> Punctuated<Field, Token![,]> = 
    body:separated<Punctuated>(
        field, 
        ",", 
        min = 1, 
        trailing = true
    )
```

## 2. Rationale

Why did we choose this verbose syntax over the concise `rule ** sep`?

### 2.1. Extensibility & Future-Proofing
Operators are rigid. If we used `**`, adding support for trailing commas would require cryptic syntax hacks (e.g., `rule ** sep,` or `rule **? sep`).
With **Parametric Rules**, adding a new feature (e.g., `max = 10` or `recover = true`) is just adding a new named argument. The breaking change risk is near zero.

### 2.2. The "Vec-Only" Trap
In systems programming (Rust), strict allocation control is vital.
* **The Operator Approach (`**`):** Implicitly promises a `Vec`. This makes it impossible to use for zero-copy parsing or streaming into other structures without double-allocation.
* **The Generic Approach (`<T>`):** Explicitly hands control to the user. It aligns with `winnow-grammar`'s philosophy of being a thin, efficient wrapper.

### 2.3. Readability (The "Strangeness Budget")
* `ident ** ","` is obscure. It resembles multiplication or globbing.
* `separated(ident, ",")` is self-documenting.
For a library that aims to be adopted by users familiar with Rust, reusing Rust's syntax for generics (`<T>`) and function arguments reduces the cognitive load significantly.

## 3. Discarded Alternatives (Anti-Patterns)

We explicitly evaluated and **rejected** the following designs. This section serves as a reference to prevent re-litigating these ideas.

### 3.1. Infix Operators (`**` / `++`)

* **Proposal:** Introduce `rule ** ","` for strict lists and `rule ++ ","` for non-empty lists.
* **Status:** **Rejected with Prejudice.**
* **Critical Flaws:**
    1.  **The Syntactic Cul-de-sac (Sackgasse):** Operators are syntactically rigid. They are binary or unary. They do not accept named arguments.
        * *Scenario:* A user needs a list that allows a trailing separator.
        * *Failure:* We cannot simply add a flag to `**`. We would be forced to invent new, cryptic syntax like `rule **? ","` or `rule **, ","`. This leads to "line noise" grammar that is hard to read and harder to extend. The `separated(...)` function, by contrast, scales indefinitely via named arguments without breaking syntax.
    2.  **Symbol Exhaustion (Verbraten):** `**` and `++` are "high-value" tokens in parser design.
        * `**` is the universal standard for **Exponentiation** in mathematics and programming (Python, Ruby, etc.).
        * `++` is the standard for **Increment** or **Concatenation**.
        * *Conflict:* By reserving `**` as a structural list operator in `syn-grammar`, we effectively ban users from using it in their own grammars (e.g., a calculator parser) without awkward escaping mechanisms. We must not pollute the user's token space for our own meta-syntax.
    3.  **The "Vec" Coupling:** As mentioned in Section 2.2, operators implicitly suggest a specific output type (usually `Vec`). This forces allocations even when the user might want to stream data or populate a pre-allocated buffer, violating Rust's zero-overhead principle.

### 3.2. EBNF Style (`{ rule, sep }`)
* **Proposal:** Use braces `{ ... }` for repetition.
* **Reason for Rejection:**
    * **Ambiguity:** In Rust/`syn` contexts, `{ ... }` strongly implies a code block or a semantic action scope. Overloading it for grammar repetition creates visual confusion and parser conflicts.

## 4. Consequences

* **For `syn-grammar` (Frontend):** The parser must be updated to handle generic arguments (`<...>`) and named arguments (`key = value`) in rule invocations. This is a one-time complexity cost.
* **For `winnow-grammar` (Backend):** This maps cleanly to `winnow::combinator::separated` and `winnow::combinator::repeat`. The backend code generation becomes simpler because it doesn't need to guess the user's intent—the arguments (`min`, `trailing`) map 1:1 to combinator configuration.
* **For Users:** Users get a "Battery-Included" experience. They don't need to write recursive boilerplate, and they don't need to learn obscure operators. They gain full control over allocation strategies (`Vec` vs `HashSet` vs `Punctuated`).
