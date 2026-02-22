# Investigation

## Test Failures and AI Refactoring (2024-03-XX)

### Issue: Test `seq_test` Failure
The test `seq_test` was failing with:
`error: Groups cannot be bound directly.`
`--> tests/comprehensive_test.rs:61:41`
`61 | ... i32) = a:("a" -> {1}) b:("b" -> {2}) -> { (a, b) }`

### Cause
Previous AI assistants refactored this test to avoid using numeric literals directly, likely due to a regression or issue with parsing numbers in the `syn` backend at that time. Instead of fixing the number parsing, they introduced anonymous groups with action blocks `("a" -> {1})` and attempted to bind variables to them `a:("a" -> {1})`, which is invalid syntax in `syn-grammar`.

### Resolution (Option 2)
We reverted the test to use `i32` directly, matching the original intent found in the git history and validating the core functionality of sequence binding.

```rust
grammar! {
    grammar seq_test {
        pub rule main -> (i32, i32) = a:i32 b:i32 -> { (a, b) }
    }
}
```

The test input was updated to `"10 20"` and expected output `(10, 20)`.

### Action Items for Future Fixes
When encountering test failures where the grammar structure seems overly complex or uses unsupported features (like bound groups):
1. Check if the test was recently refactored to avoid a core feature (like numbers).
2. Prefer restoring the original, simpler test case (e.g., `i32` binding) rather than patching the workaround.
3. If a feature (like number parsing) is truly broken, fix the feature, don't change the test to avoid it.

## Syntactic Disambiguation for Generic Rules (2026-02-22)

### ADR 003 Update: The `<_>` Marker
ADR 003 has been updated to require an explicit `<_>` (or concrete types) for rule invocations that take arguments. This was necessary to resolve an LL(1) parsing ambiguity in the `syn` macro environment.

### Potential Impact
- **Solves:** Parsing conflicts between rule calls and EBNF groups, and ensures compatibility with `syn`'s native path parsing.
- **Introduces Issues:** This is a breaking change. The previously used `rule<>(..)` syntax is no longer supported as `syn` fails to parse empty angle brackets as a valid path. All existing grammars using `rule<>(..)` will fail and must be updated to `rule<_>(..)` or `rule<Type>(..)` to comply with the new standard.
- **Scope:** Expect widespread build failures in tests and modules that rely on the old empty-bracket generic syntax.
