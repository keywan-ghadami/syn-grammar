# Analysis of `test_not_complex` Failure

This document details the investigation into the failing `test_not_complex` test and the incorrect error message it produces.

## 1. The Bug

The test `test_not_complex` in `syn-grammar/tests/peek_not_test.rs` fails.
- **Input:** `"bad"`
- **Expected Error:** `"No matching rule variant found; found unexpected token \`bad\` in rule \`main\`"`
- **Actual Error:** `"unexpected match at column 0 (line 1)\\nin main"`

## 2. Grammar Definition

The relevant grammar is `not_complex`:

```rust
grammar not_complex {
    // This rule should match any identifier, as long as it is NOT "bad".
    rule main -> () = not(bad) any:ident -> { () }

    // This rule specifically matches the token "bad".
    rule bad -> () = "bad" -> { () }
}
```

## 3. Trace Walkthrough

The test fails when parsing the input string `"bad"`. Let\'s analyze the trace output to understand the parser\'s behavior.

```
[TRACE] enter_rule: main
```
The parser begins processing the `main` rule. The first element of this rule is `not(bad)`.

```
[TRACE] enter_rule: bad
```
To evaluate `not(bad)`, the parser must first try to parse the `bad` rule as a "negative lookahead". It checks if the input *could* be parsed as `bad`.

```
[TRACE] exit_rule: bad
```
The input is `"bad"`, which perfectly matches the `bad` rule. The parser successfully parses `bad` within the lookahead.

```
[TRACE] considering_error: \'unexpected match\' (priority: 0, label: None)
[TRACE] record_error: New best error (no previous)
```
Here lies the root of the problem. The `not()` construct is designed to fail if its inner rule (in this case, `bad`) succeeds. Since the `bad` rule *did* succeed, the `not(bad)` expression correctly fails. However, upon failing, it generates a generic, low-priority error: `"unexpected match"`. This becomes the only error recorded by the parser.

```
[TRACE] exit_rule: main
```
Because the first part of the `main` rule (`not(bad)`) failed, the entire `main` rule fails, and the parser exits.

## 4. Root Cause

The final error message reported is the one that was recorded during the failure of the `not(bad)` expression.

The core issue is that the `not()` construct, when it correctly identifies a match and fails, produces a low-quality, generic error message ("unexpected match"). This generic error message is then reported to the user, masking the more meaningful, higher-level context.

The *expected* error message (`"No matching rule variant found..."`) would be generated if the parser failed to find any valid way to parse the `main` rule. However, the current implementation causes the parser to stop immediately at the `not()` failure and report its generic error, preventing the generation of a more informative error.

## 5. Next Steps

To fix this, the code generation for the `not()` pattern needs to be changed. Instead of generating a generic "unexpected match" error, it should fail without generating its own error. This would allow the parser\'s parent context (the `main` rule) to correctly report that it couldn\'t find a match for the input token `"bad"`, which would produce the expected, more helpful error message. This logic is likely located in the `syn-grammar-macros` crate.
