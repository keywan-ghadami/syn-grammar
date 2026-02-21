# ADR 007: Deprecation of Colon-Based Grammar Inheritance

**Date:** 2026-02-21
**Status:** Accepted

## Context
With the implementation of the new namespaced grammar composition system (as detailed in [ADR 006](./adr6.md)), the old mechanism for grammar reuse—colon-based inheritance (`grammar Derived : Base`)—is now obsolete.

The old system had several major drawbacks that the new system resolves:
- **No Namespace Control:** All rules from the base grammar were imported into a single, global namespace, making naming collisions a significant risk in larger projects.
- **Limited Composition:** It only supported a single level of inheritance (`A : B`), making it impossible to compose a new grammar from multiple smaller, independent modules (e.g., combining separate modules for whitespace, comments, and expressions).
- **Poor Ergonomics:** The requirement that the base grammar be an existing Rust module created tight coupling and made it less intuitive than directly including grammar definition files.

## Decision
We will deprecate the colon-based inheritance syntax (`grammar <name> : <base>`) and remove it in a future major version.

The new `include <macro_name> as <alias>;` syntax is now the official, recommended way to compose grammars. It is superior in every aspect, providing clear namespacing, multi-grammar composition, and better ergonomics.

### Deprecation Plan
1.  **Initial Warning (v0.10.0):** The parser will be updated to emit a compile-time warning (`#[deprecated]`) when it encounters the colon-based inheritance syntax, directing users to the new `include` syntax.
2.  **Full Removal (v1.0.0):** The logic for handling colon-based inheritance will be completely removed from the parser and model, making it a hard compile error.

This phased approach will give users ample time to migrate their existing grammars to the new, more powerful system while ensuring that new users adopt the best practice from the start.
