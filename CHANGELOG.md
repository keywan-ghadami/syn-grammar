# Changelog

All notable changes to this project will be documented in this file.

## [0.3.0]

### Breaking Changes
- **Renamed Built-in Parsers**:
  - `int_lit` has been renamed to **`integer`** (returns `i32`).
  - `string_lit` has been renamed to **`string`** (returns `String`).
  - This change distinguishes high-level value parsers from the low-level token parsers (`lit_int`, `lit_str`).

### Added
- **Error Recovery**: Added `recover(rule, sync_token)` to handle syntax errors gracefully by skipping tokens until a synchronization point.
- **New Built-in Parsers**:
  - `lit_int` -> `syn::LitInt`
  - `lit_char` -> `syn::LitChar`
  - `lit_bool` -> `syn::LitBool`
  - `lit_float` -> `syn::LitFloat`
  - `spanned_int_lit` -> `(i32, Span)`
  - `spanned_string_lit` -> `(String, Span)`

## [0.2.0]

### Removed
- **`include_grammar!`**: Support for external grammar files (`.g`) has been removed.
  - **Reason**: Error reporting within external files was poor, making debugging difficult.
  - **Migration**: Please move your grammar definitions inline using the `grammar! { ... }` macro to benefit from full Rust compiler diagnostics and IDE support.

### Fixed
- **Generated Code**: Fixed usage of `syn` macros (`bracketed!`, `braced!`, `parenthesized!`) by removing incorrect error propagation (`?`).
- **Generated Code**: Changed rule variant generation to use a flat list of checks instead of `else if` chains, ensuring correct "first match wins" behavior and error fallthrough.

### Internal
- **Architecture**: Extracted grammar parsing, validation, and analysis into a separate `syn-grammar-model` crate. This enables the creation of alternative backends (e.g., `winnow`) in the future.
