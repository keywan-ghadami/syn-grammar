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

## Medium Priority

### 2. Labeled Alternatives for Better Error Reporting

**Context:**
When a rule with multiple alternatives fails, the error message often defaults to the expectation of the first alternative (e.g., "expected 'let'"), even if other alternatives were possible. This is unhelpful when the user's input doesn't match any of the alternatives.

**Missing Feature:**
A mechanism to provide high-level, human-readable names for alternatives, and a new error-reporting strategy to combine these names into a summary message.

**Proposed Solution:**
1.  **Default Labels:** If an alternative is a direct call to another rule (e.g., `value = integer | boolean`), the name of the called rule ("integer", "boolean") will be used as its default label.
2.  **Explicit Labels:** For more complex, inline patterns, the author can provide an explicit label using the `# "label"` syntax. This overrides the default.
3.  **Error Logic:** When parsing a choice fails, the engine should identify all alternatives that failed with the same (or minimal) progress. It would then collect the labels of these alternatives to generate a combined error message.

**Complexity: Medium**
*   **`syn-grammar-model`**: Changes are moderate. The `parser.rs` needs to be updated to parse the `# "label"` syntax. The `Alternative` struct in `model/types.rs` needs a new `label` field. A new pass in `analysis.rs` would be needed to determine the default labels.
*   **`syn-grammar-macros`**: This is where the main complexity lies. The code generation in `codegen/pattern.rs` for alternatives (`|`) must be significantly reworked. Instead of failing immediately, the generated parser would need to speculatively try all alternatives, store their potential errors and labels, compare them based on which one made the most progress, and then combine the labels to create a new summary error. This fundamentally changes the runtime error handling logic.

### 3. Implicit Token Literals / Token Aliases

**Context:**
Users must repeatedly use string literals for common tokens (e.g., `"+"`, `"fn"`).

**Missing Feature:**
A standard prelude or alias system for common Rust tokens, or implicit support for single-char tokens.

**Proposed Syntax:**
*   **Implicit Tokens:** Support `char` literals (e.g., `'+'`) to denote single-token punctuation.
*   **Prelude:** Standard aliases like `PLUS`, `MINUS`, `DOT`.

**Complexity: Low**
*   **`syn-grammar-model`**: All changes are confined here. The parser needs to be updated to accept `char` literals and convert them to string literals. For aliases, the parser would check an identifier against a predefined list of "prelude" tokens and convert it to a `ModelPattern::Lit` instead of a `RuleCall`. This is a localized change in the parsing and validation logic.
*   **`syn-grammar-macros`**: No changes are required. The codegen crate would receive a `ModelPattern::Lit`, which it already knows how to handle. The complexity is abstracted away in the model creation phase.
