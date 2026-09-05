# Changelog

All notable changes to this project will be documented in this file.

## [0.9.0] - Draft, unreleased

> This version is **not on crates.io yet**. The last published release is
> 0.8.0; everything below is the net change against 0.8.0.

The theme of this release is **error messages**: a new diagnostics engine
selects the most informative failure by progress, then priority, and every
message names the expected token, the found token, the position and the chain
of rules. The generated parsers now run in **linear time**, and grammars are
composed through explicit `import` / `extern rule` interfaces.

For end users who only write `grammar! { … }` and call the generated
`parse_X(ParseStream) -> syn::Result<T>`, the wrapper signature is unchanged.
What changes for them is listed under **Grammar DSL** below; the rest concerns
hand-written parsers plugged into a grammar, code that matches error text, and
backend authors.

### Breaking Changes

**Grammar DSL**

- **Grammar inheritance (`grammar Derived : Base`) is removed.** It worked
  through an implicit `use super::Base::*;`, which switched off the "Undefined
  rule" check and the shadowing analysis.
  - **Migration**: `import Base as base;` and call the rules as `base::num`.
    The old form is rejected at the `:` with a message that says so.
- **A function brought in with `use` can no longer be called like a rule**
  ("import injection", documented in 0.8.0).
  - **Migration**: declare it: `extern rule my_parser -> ReturnType;`. The
    "Undefined rule" error now names this replacement, and says explicitly
    that a `use` of the same name is not enough. The function signature also
    changes, see the runtime API section.
- **Uppercase rule names are lexical.** No whitespace is allowed between the
  patterns of a rule whose name starts with an uppercase letter.
  - **Impact**: an existing grammar with an uppercase rule changes behaviour
    *silently*. Rename the rule or wrap its body in `spaced(...)`.
- **`separated(..., trailing=false)`** no longer fails hard on a separator with
  no item after it; it backtracks to before the separator.
  - **Impact**: grammars of the form `paren(list? ","?)` now work; grammars
    that relied on the hard error now accept the input.
- **Unknown named arguments of `separated`/`repeated` are a compile error.**
  They used to be ignored; `error=` was even documented once and never did
  anything. Supported: `min`, `trailing` (separated only), `item_label`.
- **`any_ident` accepts keywords** (`Ident::parse_any`) and is no longer
  equivalent to `ident`.
  - **Impact**: anyone who relied on `any_ident` rejecting a keyword must
    switch to `ident`.
- **The `<_>` disambiguation for rule arguments is removed.** A rule call with
  arguments uses named arguments (`rule(name=arg)`) or generic parameters to
  distinguish it from a grouping. `fail`, `peek`, `not`, `separated` and
  `repeated` are unaffected.
- **`syn_grammar::Identifier` and `syn_grammar::StringLiteral` are no longer
  re-exported at the crate root.**
  - **Migration**: `syn_grammar::types::Identifier`, `::types::StringLiteral`.
- **The `include_grammar` macro is deleted.** Since 0.2.0 it only emitted an
  explanatory `compile_error!`; now the compiler reports "cannot find macro".

**Error messages** (anything that matches on message text is affected)

- **Selection compares progress first**, then fatality, then priority. An
  early `fail("…")` loses against an ordinary error that got further into the
  input; at the *same* position `fail` still wins.
- **The rule context is a suffix**, one line per rule, innermost first, with
  underscores turned into spaces: `\nin inner rule\nin outer rule` instead of
  the prefix `Error in rule 'inner': …`. ` at column N (line M)` is appended
  when the span carries position data.
- **Many fixed texts changed**, among them `No matching rule variant found`
  -> ``expected one of: `a`, `b`; found unexpected token `x` ``,
  `expected at least N items` -> `expected at least N <item>s, found M`,
  `unexpected match` -> ``unexpected match for rule `X`; found `Y` in rule `Z` ``.
  The binding catalogue is `docs/adr/adr13-error-message-contract.md`.

**Runtime API** (`syn_grammar::rt`, hand-written parsers, backend code)

- **A hand-written rule takes `&rt::Stream<'a>` and returns
  `rt::StreamResult<'a, T>`.** `Stream<'a>` is `syn::parse::ParseBuffer<'a>`
  and `StreamResult<'a, T>` is `Result<T, ParseError<'a>>`. The 0.8.0 form
  `fn(ParseStream) -> syn::Result<T>` is not accepted any more. Deliberately
  `&ParseBuffer<'a>` and not syn's alias `ParseStream<'a> = &'a ParseBuffer<'a>`:
  the alias would shorten `'a` to the stack frame on a fork, and errors from a
  fork could not leave the call.
- **`ParseError<'a>` replaces `syn::Error` inside the engine.** It carries the
  span (for display), the cursor (for progress comparison), a priority
  (`PRIO_NORMAL` / `PRIO_LABELED` / `PRIO_AGGREGATED` / `PRIO_STRUCTURAL`),
  the fatality flag of the cut, the rule stack, and the expectation set
  `expected` (`ParseError::expecting`, `with_expected`). Action blocks may
  still fail with `syn::Error`; it converts. `finish_variants` takes the name
  of the scope end as a fifth argument.
- **`ParseContext` is `ParseContext<'a>`.** Removed: `set_fatal`,
  `check_fatal`, `trigger_fail`, `record_error`, `take_best_error`,
  `is_best_error_deep`, `rule_stack()`, and `define` / `is_defined` /
  `enter_scope` / `exit_scope` directly on the context (now under
  `ctx.scopes`). New: `record_failure`, `absorb`, `best_error`, `furthest`,
  `enter_rule` / `exit_rule`, `enter_group` / `exit_group`,
  `end_of_scope_msg`, `mode_stack`, `group_depth`. `record_span` returns
  `syn::Result<()>`. Fatality lives on the error (`ParseError::is_fatal`).
- **The 0.8.0 combinators are gone**: `attempt`, `peek`, `not_check`,
  `attempt_recover`, `parse_ident`, `parse_int`, `skip_until`; they took a
  `ParseStream`. Their counterparts are `peek_syn`, `finish_variants`,
  `parse_separated`, `parse_repeated` and the module `stream`: `parse_syn`,
  `parse_with`, `fork` / `advance_to` (backtracking), `group`, `step`,
  `take_token`. After an error the stream may have advanced; whoever wants to
  backtrack works on a fork.
- **`builtins::parse_*_impl` have the stream signature** described above. The
  trait `CommonBuiltins` and its `impl for ParseStream` are deleted.
  `token_filter::{alpha, alphanumeric, digit, hex_digit, oct_digit}` are
  cursor primitives (`Cursor<'a> -> ParseResult<'a, X>`) run via `rt::step`.
- **`grammar-kit::testing::TestResult<T, E>` has a third parameter `S = ()`**,
  the `'static` bound on `E` is gone, `assert_failure_contains` /
  `assert_failure_not_contains` return `Self`, and `Testable` is generalised
  from `syn::Result<T>` to `Result<T, E>`.
- **`syn-grammar-model`**: `parse_grammar_with_builtins` is replaced by
  `parse_grammar::<B: Backend>`; `model` no longer re-exports `backend::*` and
  `types::*` flatly; `GrammarDefinition` loses `inherits` and gains
  `extern_rules` / `imports`; `Rule` gains `return_type_kind` and
  `is_lexical`; `RuleCall.rule_name` is `rule_path`; `params` is
  `Vec<RuleParameter>`. Details in `core/syn-grammar-model/CHANGELOG.md`.
- **`grammar-kit`**: the features `rt` and `trace` are removed (they switched
  nothing on), as are the never-working macro `test_both_backends!` and the
  never-included module `transaction`.
- **Repository layout**: a virtual workspace with `syn-grammar/` and `core/`;
  library roots are named after their crate (`syn_grammar.rs`,
  `grammar_kit.rs`, …). Relevant for contributors only.

### Added

**Grammar DSL**

- **Labelled alternatives (`# "label"`)**: an alternative that fails at its
  start is reported by its label, and several such alternatives are
  aggregated into `expected one of: a number, a string`. An alternative that
  fails after consuming input keeps its own, more specific message.
- **`fail("message")`**: fails with the given text at high priority.
- **`separated(item, sep)` and `repeated(item)`** with the named arguments
  `min`, `trailing` and `item_label`, and an optional container type
  (`separated<HashSet>(…)`, default `Vec`). `item_label` names and counts the
  items in error messages: `expected function argument … in function argument 2`.
- **`extern rule`** for hand-written parsers and **`import … as alias;`** for
  the rules of another grammar (`alias::rule`), before or inside the block.
- **`lex(...)` and `spaced(...)`** for explicit whitespace sensitivity, plus
  the uppercase convention for lexical rules.
- **`until(terminator)`** collects raw tokens up to a terminator without
  consuming it; **`count(pattern)`** returns how often a pattern matched.
- **Group bindings**: `var:("a" | "b")` binds the result of a group.
- **Named and generic arguments in rule calls**: `rule(key=value)` and
  `rule<T>(...)`.
- **Simplified rule syntax**: the `rule` keyword, the return type and the
  action block are optional.
- **New built-ins**: `any_ident`, `pat`, `inner_attrs`, `lit_byte`,
  `named_field`, `unnamed_field`, `visibility`, `generics`, `return_type`,
  `statements`. Any `syn::` type with `impl Parse` can be used by its path
  (`x:syn::Expr`). `pat` closes the biggest gap: `syn::Pat` has no `impl Parse`.
- **`DEBUG_GRAMMAR=1`** prints the generated code to stderr during the build.

**Engine**

- **Linear parsing (ADR 15).** No parse step materialises the remaining
  input. A `syn::Type` costs `input.parse::<T>()` instead of a new
  `TokenBuffer` over the rest of the enclosing delimiter group; single tokens
  are read in O(1); `peek_syn` allocates nothing. Measured on an argument
  list with 2000 entries: 1.174 s -> 5.33 ms, and twenty times the input now
  costs 16x instead of 356x.
- **Diagnostics engine.** Progress is compared via `syn::buffer::Cursor`
  (a pointer comparison, independent of the compiler version), then fatality,
  then priority. A **high-water mark** (`ParseContext::furthest`) keeps the
  error that a *successful* backtrack (`?`, `*`, `separated(min=0)`) would
  otherwise discard. A **live rule stack** gives remembered errors their rule
  context. `unexpected end of group` and `unexpected end of input` are told
  apart.
- **`with_span` derive macro** and the `WithSpan` trait.
- **`cxx-parser`**, a parser for the `cxx` bridge syntax, as the acceptance
  benchmark for error quality (its own crate, 0.1.0, not published).
- **Self-hosting test** (`tests/self_hosting_test.rs`): the grammar DSL
  written in the grammar DSL, parsing the documentation's grammars and
  checking the diagnostics a grammar author gets. The second acceptance
  benchmark next to `cxx-parser`.

### Fixed

- **`expected one of:` lists every alternative.** A branch that starts with a
  built-in or with another rule was invisible in the enumeration, and a
  delimiter appeared under its internal name: `factor = i:i32 | paren(…)` on
  `*` reported ``expected `Paren` ``. It now reports
  ``expected one of: `integer literal`, `parentheses`; found unexpected token `*` ``.
  Errors carry an expectation set (`ParseError::expected`) that the
  alternative chain unions, through nested rules; a label still replaces the
  inner list. At the end of the input or of a group the enumeration carries
  the `unexpected end of …, ` prefix. A single built-in keeps syn's wording.
- **A numeric literal used as a token** (`"0"`) is a pinned compile error
  that names the built-ins to use (`i32`, `u64`, `lit_int`); the message used
  to recommend `integer`, which does not exist. The parked test
  `digits.fixme` became `tests/ui/numeric_literal_token.rs`.
- **`digit`, `hex_digit`, `oct_digit`** were catalogued as `syn::Ident` but
  return `syn::LitInt`; a generic rule instantiated with them produced a
  compiler error in generated code.
- **The "Undefined rule" check** was switched off by *any* `use` statement; it
  now hangs only on glob imports. A typo in a rule name is reported at the
  call site again instead of as a follow-up error in generated code.
- **A `syn::` type without `Parse`** (e.g. `syn::Field`) produced a raw
  trait-bound error on generated code. The message now appears on the user's
  line and names the built-in to use instead.
- **Errors of discarded alternatives were lost** when a later alternative
  succeeded and left input behind; only syn's `unexpected token` remained.
- **Multi-character operators** (`::`, `->`, …) were matched per character
  with a span adjacency check that was ineffective inside a real procedural
  macro before Rust 1.88, so `a : : b` passed as `::`. They are now syn's
  joint tokens.
- **Zero-progress guard** in `*`, `+` and the list combinators: a pattern
  that consumes nothing no longer loops forever.
- **`lex(...)` / `spaced(...)`** did not restore the whitespace mode on error.
- **The validator forbade arguments for built-in rules.**

### Documentation

- `SYNTAX.md` is the complete reference again: rule forms, `use` statements,
  attributes, multi-token literals, the full built-in catalogue, container
  types for lists, backtracking, the error-message operators, `extern` /
  `import` with a migration note for inheritance, and `DEBUG_GRAMMAR`.
- New: `docs/ERROR_HANDLING.md` (how the engine picks a message),
  `docs/adr/adr13-error-message-contract.md` (the binding catalogue, every
  point with its test), `docs/adr/adr15-linear-parsing.md`, `GOALS.md`,
  `ARCHITECTURE.md`. `EXTENDING.md` was removed: it described an API
  that no longer exists; `ARCHITECTURE.md` covers the same ground.
- `#![warn(missing_docs)]` in all crates; every public item is documented.
- The minimum supported Rust version is **1.88** (`rust-version`), the first
  version in which spans carry positions inside a procedural macro on stable.

## [0.8.0]

### Added
- **Literal Bindings**: Added support for binding string literals directly to variables in grammar rules (e.g., `label:"literal"`). This resolves to the `syn::Token` corresponding to the literal.
- **Optional Literal Bindings**: Added support for optional literal bindings (e.g., `label:"literal"?`, which resolve to `Option<Token>`.
- **Span Binding on Literals**: Extended the span binding syntax (`@`) to support string literals (e.g., `"literal" @ span_var`), allowing direct capture of a literal's span.

### Breaking Changes
- **Backend API**: The internal data model and macro infrastructure were updated to support the new literal binding features. This constitutes a breaking change for downstream backend implementers (e.g., `winnow-grammar`) who must now handle these new syntax-tree nodes. This change is not breaking for end-users who only write grammars.

## [0.7.1]

### Fixed
- **Typed Parameter Validation**: Fixed a bug where rule parameters with explicit types (e.g., `rule list<T>(item: Type)`) were incorrectly flagged as "Undefined rule" by the validator. The validator now correctly recognizes all parameters, regardless of whether they have explicit types or not.

## [0.7.0]

### Added
- **Portable Primitives**: Introduced a distinction between `PORTABLE_BUILTINS` (`ident`, `integer`, `alpha`, etc.) and `SYN_SPEC_BUILTINS` (`rust_type`, `lit_str`, etc.). This clarifies the portability contract for authors of alternative backends (e.g., `winnow-grammar`), encouraging a rich, shared vocabulary of common parsing concepts.
- **Portable Types**: Introduced backend-agnostic wrapper types `Identifier`, `StringLiteral`, and `SpannedValue<T>`. These types implement `ToTokens`, allowing them to be used seamlessly in `quote! { ... }` macros while provides a consistent API across different backends.
- **Numeric Built-ins**: Added a comprehensive set of portable numeric built-ins:
    - **Signed Integers**: `i8`, `i16`, `i32`, `i64`, `i128`, `isize` (and `int*` aliases).
    - **Unsigned Integers**: `u8`, `u16`, `u32`, `u64`, `u128`, `usize` (and `uint*` aliases).
    - **Floating Point**: `f32`, `f64`.
    - **Alternative Bases**: `hex_literal`, `oct_literal`, `bin_literal` (parses into `u64`).
- **Spanned Primitives**: Added `spanned_` variants for all primitives (e.g., `spanned_i32` returns `SpannedValue<i32>`), allowing easy access to source location data.
- **`whitespace` Primitive**: Added the `whitespace` assertion, which ensures a gap (non-adjacency) between two tokens.
- **Lookahead Operators**: Added support for positive (`peek(...)`) and negative (`not(...)`) lookahead operators.
    - `peek(pattern)`: Succeeds if the pattern matches, but does not consume input.
    - `not(pattern)`: Succeeds if the pattern does *not* match. Does not consume input.
- **`alpha` Primitive**: Added the `alpha` built-in primitive, which matches an identifier composed entirely of alphabetic characters.
- **Architecture**: Introduced `Backend` trait and `CommonBuiltins` to decouple the grammar definition from the `syn` implementation, paving the way for other backends.
- **ADR for Primitives**: Added an Architecture Decision Record (`docs/adr/adr1.md`) to document the design for handling character-level, byte-level, and token-level primitives across different backends.
- **ADR for Portable Types**: Added an Architecture Decision Record (`docs/adr/adr2.md`) to document the design for portable types and explicit backend contracts.
- **Higher-Order Generic Rules**: Added support for generic rules with untyped grammar parameters (patterns) and generic type parameters (e.g., `rule list<T>(item) -> Vec<T>`).
- **Monomorphization**: Implemented compile-time monomorphization of generic rules, allowing the creation of reusable grammar patterns without runtime overhead.
- **Generic Arguments in Rules**: Generic parameters now support standard Rust trait bounds, which are enforced on the inferred types.
- **Numeric Argument Support**: Updated the parser to support numeric literals as arguments to rules (e.g., `value(10)`), enabling more flexible rule parameterization.
- **ADR for Generic Rules**: Added an Architecture Decision Record (`docs/adr/adr3.md`) documenting the design of higher-order generic rules and macro-time monomorphization.
- **Restored Tests**: Added back `test_rule_arguments` and `test_multiple_arguments` to ensure rule parameter functionality works as expected.
- **Shadowing Detection**: Compile-time validation now reports errors when a grammar rule alternative is shadowed by a previous prefix alternative or is an exact duplicate.

### Changed
- **Backend-Agnostic Model**: The `syn-grammar-model` crate now exposes `parse_grammar_with_builtins`. This allows backend authors to validate grammars against their own set of built-in rules.
- **Backend Author Guide**: `EXTENDING.md` has been rewritten to focus on how to build custom parser generator backends using `syn-grammar` as the frontend DSL.
- **Validation Errors**: Shadowing and duplicate alternatives are now treated as hard errors instead of warnings to ensure grammar correctness.

### Fixed
- **Repetition Syntax**: Fixed a regression where repetition patterns were incorrectly requiring brackets `[...]` instead of parentheses `(...)`.
- **Linter Warnings**: Resolved multiple `clippy` warnings (unused variables, collapsible if-blocks, approximate constants).
- **Float Testing**: Improved float primitive tests to use proper epsilon comparison for accuracy.

### Breaking Changes
- **Portable Types for Primitives**: To improve backend portability (see ADR 2), several built-in parsers now return backend-agnostic types instead of `syn`-specific ones.
    - `ident` now returns `syn_grammar::Identifier` instead of `syn::Ident`.
    - `string` now returns `syn_grammar::StringLiteral` instead of `String`.
    - **Impact**: Action blocks that expect the previous `syn` types must be updated to use the new portable types (e.g., use `name.text` instead of `name.to_string()` or rely on `Display` impl).
- **Renamed `Spanned<T>` to `SpannedValue<T>`**: The `Spanned<T>` struct has been renamed to `SpannedValue<T>` to avoid name collisions with the `syn::spanned::Spanned` trait.
    - **Impact**: Code that uses `Spanned<T>` (e.g. return types of `spanned_*` built-ins) must be updated to use `SpannedValue<T>`.
- **Built-in Rule Resolution**: The precedence of built-in rules (like `ident`, `string`) has changed. They are no longer hardcoded keywords but are now provided as default implementations in `syn_grammar::builtins`.
    - **Impact**: If you define a rule named `ident` in your grammar, it will now *shadow* the built-in `ident` parser instead of being ignored. This fixes a long-standing limitation but may change behavior if you accidentally relied on the shadowing being ignored.

## [0.6.0]

### Added
- **`use super::*`**: The generated parser module now includes `use super::*;` by default, allowing parsers to seamlessly access other items defined in the parent module.
- **Use Statement Support**: Added support for standard Rust `use` statements within the grammar block (e.g., `use syn::Ident;`). These are passed through to the generated parser module.

## [0.5.0]

### Added
- **Span Binding Syntax**: Added support for the `name:parser @ span_var` syntax. This allows binding the result of a parser to `name` and its span to `span_var` simultaneously (e.g., `id:ident @ span`).

### Deprecated
- **Spanned Literal Parsers**: The `spanned_*_lit` built-in parsers (e.g., `spanned_int_lit`, `spanned_string_lit`) are deprecated. Use the standard literal parsers with the new span binding syntax instead (e.g., `lit_int @ span`).

## [0.4.0]

### Added
- **Token Recognition in Literals**: Enhanced parsing of string literals in the grammar to support multi-token sequences and complex combinations (e.g. `"?."`, `"@detached"`).
- **Pretty Error Printing**: The testing framework now pretty-prints `syn::Error` with source code context and underlining when assertions fail.
- **Outer Attributes**: Added support for parsing outer attributes (`#[...]`) via the `outer_attrs` built-in.
- **Span Binding**: Added support for capturing spans via `rule @ span_var` syntax.

### Improved
- **Error Spans**: Generated code now uses specific token spans instead of `Span::call_site()` where possible, resulting in more precise error highlighting in IDEs.

### Fixed
- **Documentation**: Fixed failing doctests in README, cleaned up examples, and clarified usage of brackets and delimiters.

### Internal
- **Testing**: Stabilized testing infrastructure.

## [0.3.0]

### Breaking Changes
- **Runtime Dependency**: Generated parsers now depend on the new `grammar-kit` crate (formerly `syn-kit`). Users must add `grammar-kit = "0.3.0"` to their `Cargo.toml`.
- **Renamed Built-in Parsers**:
  - `int_lit` has been renamed to **`integer`** (returns `i32`).
  - `string_lit` has been renamed to **`string`** (returns `String`).
  - This change distinguishes high-level value parsers from the low-level token parsers (`lit_int`, `lit_str`).

### Added
- **Attributes on Rules**: Rules can now be decorated with attributes, such as doc comments (`///`) or `#[cfg(...)]`.
- **Error Recovery**: Added `recover(rule, sync_token)` to handle syntax errors gracefully by skipping tokens until a synchronization point.
- **Rule Arguments**: Rules can now accept parameters (e.g., `rule value(arg: i32) -> ...`), allowing context to be passed down the parser chain.
- **Grammar Inheritance**: Grammars can inherit from other modules (e.g., `grammar MyGrammar : BaseGrammar`), enabling the use of external or manually written "custom parsers".
- **Testing Utilities**: Added `syn_grammar::testing` module with fluent assertions (`assert_success_is`, `assert_failure_contains`) to simplify writing tests for grammars.
- **Improved Error Reporting**: The parser now prioritizes "deep" errors (errors that occur after consuming tokens) over shallow errors.
- **New Built-in Parsers**:
  - `lit_int` -> `syn::LitInt`
  - `lit_char` -> `syn::LitChar`
  - `lit_bool` -> `syn::LitBool`
  - `lit_float` -> `syn::LitFloat`
  - `spanned_int_lit` -> `(i32, Span)`
  - `spanned_string_lit` -> `(String, Span)`
  - `spanned_float_lit` -> `(f64, Span)`
  - `spanned_bool_lit` -> `(bool, Span)`
  - `spanned_char_lit` -> `(char, Span)`

### Internal
- **Architecture**: Extracted runtime utilities (backtracking, error reporting, testing) into a separate `grammar-kit` crate.

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
