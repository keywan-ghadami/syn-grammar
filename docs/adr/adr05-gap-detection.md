# ADR-05: Unambiguous Syntax for Rule Calls, Grouping, and Delimiters

**Status:** Accepted

**Context:**

The grammar DSL has a critical three-way ambiguity between:
1.  **Rule Call with Arguments:** `my_rule(a, b)`
2.  **Precedence Grouping:** `("a" | "b")` which controls parser logic but does not consume `()`.
3.  **Delimiter Matching:** A pattern that consumes a literal `(` and `)` from the input stream.

A single syntax like `(...)` for both grouping and delimiter matching is fundamentally ambiguous, especially in complex patterns involving repetition, such as `( (a) )*`. The parser cannot reliably infer the user's intent. Previous design iterations attempting to unify this syntax were flawed.

**Decision:**

To eliminate all ambiguity, we will enforce a clear, explicit syntax for each distinct operation. This design prioritizes correctness and readability over minimal verbosity.

1.  **Rule Calls:** All parameterized rule calls **must** use a generic-style syntax, `rule<...>(...)`. The presence of `<...>` unambiguously marks it as a function-like call.
    - `my_rule<>(arg)`: Call a non-generic rule with arguments.
    - `list<T>(item)`: Call a generic rule with type and value arguments.
    - A rule name *without* `<...>` is a simple, non-parameterized call.

2.  **Precedence Grouping:** The `( ... )` syntax is used **exclusively** for logical grouping to control the order of operations for `|`, `*`, `+`, and `?`. It does not consume parentheses from the input.
    - `("a" | "b") "c"`: Correctly parses `a c` or `b c`.

3.  **Delimiter Matching:** To match literal delimiters in the input stream, the following syntax is used:
    - `paren(...)`: Matches a literal `(` and `)` in the input.
    - `[ ... ]`: Matches a literal `[` and `]` in the input.
    - `{ ... }`: Matches a literal `{` and `}` in the input.

The special `paren(...)` keyword is necessary because `()` is the only delimiter that would otherwise be ambiguous with precedence grouping.

**Examples of the Final Syntax:**

- `rule call_with_args = my_rule<>(1, 2);`
- `rule group_for_or = ("a" | "b");`
- `rule match_a_tuple = paren(i32, i32);`
- `rule repeated_tuples = (paren(i32))*;` // Correctly combines grouping and delimiter matching

This design provides a robust and intuitive syntax for the grammar DSL, ensuring that every pattern has one and only one meaning.
