# Changelog

All notable changes to this project will be documented in this file.

## [0.9.0]

### Breaking Changes: New Unambiguous Syntax (ADR 005)

This release introduces a new, explicit, and unambiguous syntax for rule calls, grouping, and delimiter matching to resolve critical parser ambiguities. This is a significant breaking change that improves grammar clarity and correctness.

- **Rule Call Syntax**: All parameterized rule calls **must** now use a generic-style syntax: `rule<...>(...)`.
    - `my_rule<>(arg)`: For non-generic rules with arguments. The `<>` is mandatory.
    - `list<T>(item)`: For generic rules.
    - This replaces the old whitespace-sensitive (`rule(arg)`) and macro-style (`rule!(arg)`) syntaxes.

- **Grouping vs. Delimiter Syntax**: The syntax for precedence grouping and matching literal parentheses has been disambiguated.
    - `( ... )`: Used **exclusively** for logical grouping to control operator precedence (`|`, `*`, `+`, `?`). Does *not* match `()` in the input.
    - `paren( ... )`: New syntax **exclusively** for matching a literal `(` and `)` in the input stream.
    - This separation resolves a fundamental ambiguity, making complex nested and repeated patterns possible and reliable.

### Added
- **Consolidated Error Messages with Labeled Alternatives**: The parser can now produce a single, clear error message when multiple alternatives fail at the same position (e.g., "expected one of: an expression, a statement").
- **High-Priority Manual Error Reporting**: Added the `fail!("message")` built-in rule, which always fails with a custom error message that takes precedence over other parsing errors.
- **Parametric List Rules (ADR 004)**: Added `separated` and `repeated` built-in rules for concise list parsing, using the new `separated<...>(...)` syntax.
- **Until**: Added support for the `until` pattern (e.g., `body:until(";")`), which consumes tokens until a terminator pattern is found.

### Removed
- **Old Rule Call Syntaxes**: Whitespace-sensitive (`rule(arg)`) and macro-style (`rule!(arg)`) syntaxes are removed.
- **Ambiguous Delimiter Matching**: Using `( ... )` to match literal parentheses is no longer supported. Use `paren( ... )` instead.

## [0.8.0]

### Added
- **Literal Bindings**: Added support for binding string literals directly to variables in grammar rules (e.g., `label:"literal"`).
- **Optional Literal Bindings**: Added support for `label:"literal"?`, which resolves to `Option<Token>`.
- **Span Binding on Literals**: Extended `@` to support string literals (e.g., `"literal" @ span`).

### Breaking Changes
- **Backend API**: The internal data model was updated for literal binding features. This is a breaking change for backend implementers.

## [0.7.1]

### Fixed
- **Typed Parameter Validation**: Fixed a bug where rule parameters with explicit types were incorrectly flagged as "Undefined rule".

... (Older versions remain unchanged)
