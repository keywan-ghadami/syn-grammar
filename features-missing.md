# Missing Features for Intuitive Grammar Definitions

This document outlines features that would make `syn-grammar` significantly more intuitive and ergonomic.

## High Priority

### 1. Support for "Extern" or Imported Rules

**Context:**
Complex grammars often span multiple files or need to reuse rules from other crates. While `use` statements are supported, there is no dedicated syntax to declare that a rule is "external" and should not be generated but rather expected to exist in the scope or be imported from a specific location.

**Missing Feature:**
Explicit support for declaring external rules or importing entire grammar modules.

**Proposed Syntax:**
*   **`extern rule name -> Type;`**: Declares a rule that is implemented elsewhere (e.g., manually implemented or in another module).
*   **`import grammar Path;`**: Imports all rules from another grammar definition.

**Complexity: High**
*   **`syn-grammar-model`**: Requires significant changes to `parser.rs` to handle the new `extern` and `import` keywords. The grammar model in `model/types.rs` needs new structures to represent these concepts. The most complex part is in `validator.rs`, which would need to handle grammar dependencies and path resolution for imports, a non-trivial task in a proc-macro environment.
*   **`syn-grammar-macros`**: The codegen logic in `codegen/rule.rs` would need to be updated to skip generating `extern` rules and to correctly bring imported rules into scope, likely via `use` statements. This feature introduces a new concept of inter-grammar dependencies that affects the entire compilation pipeline.
