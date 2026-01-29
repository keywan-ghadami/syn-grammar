# Changelog

All notable changes to this project will be documented in this file.

## [0.2.0]

### Removed
- **`include_grammar!`**: Support for external grammar files (`.g`) has been removed.
  - **Reason**: Error reporting within external files was poor, making debugging difficult.
  - **Migration**: Please move your grammar definitions inline using the `grammar! { ... }` macro to benefit from full Rust compiler diagnostics and IDE support.
