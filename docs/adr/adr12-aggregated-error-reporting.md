# ADR 12: Aggregated Error Reporting with Abstraction

## Status
Proposed

## Context
Currently, the parser uses a "Deepest Error Wins" strategy. When multiple parsing branches fail, the one that progressed furthest in the input stream is reported. If multiple branches fail at the same position, the one with the deepest rule stack or higher priority wins.

This approach has limitations in greedy repetitions or optional branches. For example, given:
```rust
rule main = letter+ num_word+ eof
```
And input `a b one c`.
1. `letter+` consumes `a`, `b`.
2. `num_word+` consumes `one`.
3. At `c`, the parser expects either another `num_word` OR `eof`.

Currently, the parser attempts `num_word`, fails inside it (deep stack), and reports "expected one of: zero, one, two". The `eof` expectation (shallower stack) is discarded. The user sees a specific error that might be misleading if they intended to finish the input.

Furthermore, listing all raw tokens for complex rules (e.g., "expected one of: zero, one, two, ... 50 others") is verbose and unreadable.

## Decision
We will move from a "Single Best Error" model to an **Aggregated Error Model** that collects all valid expectations at the failure position.

### 1. Error Aggregation
Instead of storing a single `best_error`, the `ParseContext` will maintain a list of `best_errors` occurring at the furthest position found so far.
*   If a new error occurs at a **deeper** position, it clears the list and starts a new one (Progress).
*   If a new error occurs at the **same** position, it is added to the list (Accumulation).
*   Priorities (Explicit Fail > Fatal > Normal) still apply to filter the list at that position.

### 2. Hierarchical Abstraction (Rule Names vs. Tokens)
To avoid spamming the user with token lists, we will use the Rule Name as the expectation under specific conditions:
*   **Simple Rules:** If a rule fails and it consists of a small set of literals (e.g., < 5), we can expand them ("expected one of: a, b, c").
*   **Complex Rules:** If a rule is complex or has many variants, the error should report the high-level intent: "expected `num_word`".
    *   This requires `syn-grammar` to tag rules or analyze complexity during generation.
    *   Alternatively, we can use the `label` field. If a rule has a label (e.g., `rule num_word # "number word"`), that label takes precedence over the inner tokens.

### 3. Error Message Formatting
The final error message will be composed of:
1.  **Primary Expectation:** "expected one of: `num_word`, `end of input`"
2.  **Found:** "but found `c`" (Standard `syn` behavior, but explicit context helps).
3.  **Hints (Optional):** Detailed breakdown of complex rules can be provided as a help message or "hint".
    *   Example: "Hint: `num_word` matches one of: `zero`, `one`, `two`..."

### 4. Implementation Strategy
*   **ParseContext:** Update to hold `Vec<ErrorState>`.
*   **Record Error:** Logic to append to vector if positions match.
*   **Take Best Error:** Logic to deduplicate and format the list of errors into a single `syn::Error` message.
*   **Codegen:** Ensure `eof` and other terminals record errors that can be aggregated.

## Consequences
### Positive
*   **Completeness:** The user sees all valid options to fix their code (e.g., "I could type another number OR I could stop here").
*   **Readability:** Abstracting complex rules prevents giant "one of" lists.
*   **Correctness:** Fixes the issue where greedy loops hide the valid exit condition.

### Negative
*   **Complexity:** The `ParseContext` logic becomes more complex (merging, deduplicating).
*   **Performance:** Slight overhead in collecting and formatting multiple errors, though this only happens on failure paths.
*   **Ambiguity:** "expected `Expression`" might be too vague if the user doesn't know what an expression is (mitigated by Hints).

## Example
Input: `a b one c`
Old Output: `in rule main: in rule num_word: expected one of: zero, one, two`
New Output: `expected one of: num_word, end of input`
(Optional Hint: `num_word` covers `zero`, `one`, `two`)
