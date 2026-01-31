# Changelog

All notable changes to this project will be documented in this file.

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
