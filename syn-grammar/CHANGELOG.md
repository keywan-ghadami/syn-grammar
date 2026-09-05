# Changelog

All notable changes to this project will be documented in this file.

## [0.9.0] - Draft, unreleased

> This version is **not on crates.io yet**. The last published release is
> 0.8.0; everything listed here is to be read against 0.8.0.

The error diagnostics were rebuilt. Along the way the engine moved from the
`ParseStream` model to functional cursor parsing and, with ADR 15 stage 3, back
onto a stream - this time with the new diagnostics and without the
materialisation bridge. Net effect against 0.8.0: the same stream type, a new
error selection, linear running time. This section summarises about 280 commits
that had not made it into the changelog in the meantime.

For end users who only write `grammar! { … }` and call the generated
`parse_X(ParseStream) -> syn::Result<T>`, the move is small: the signature of
that wrapper is unchanged. Affected are mainly hand-written parsers plugged into
a grammar, and anything that checks concrete error message text.

### Breaking Changes

- **Crate and file layout.** The repo is a virtual workspace: `syn-grammar`
  lives under `syn-grammar/`, `grammar-kit` and `syn-grammar-model` under
  `core/`. All `lib.rs`/`mod.rs` files are named after their module
  (`grammar_kit.rs`, `syn_grammar.rs`, `model.rs`, `codegen.rs`) and wired in
  via `[lib] path`.
  - **Impact**: only relevant for contributors, not for users of the crates.

- **`ParseError<'a>` replaces `syn::Error` as the internal error type.** New:
  `ParseResult<'a, T> = Result<(T, Cursor<'a>), ParseError<'a>>` and the
  priority constants `PRIO_NORMAL`/`PRIO_LABELED`/`PRIO_AGGREGATED`/`PRIO_STRUCTURAL`.
  - **Impact**: the type carries a lifetime. Anyone naming it in their own
    signatures has to thread it through.

- **`ParseContext` is now `ParseContext<'a>`.** Removed: `set_fatal`,
  `check_fatal`, `trigger_fail`, `record_error`, `take_best_error`,
  `is_best_error_deep`, `define`/`is_defined`/`enter_scope`/`exit_scope`
  directly on the context, `rule_stack()`. New: `record_failure`, `absorb`,
  `best_error`, `furthest`, `enter_rule`/`exit_rule`, `enter_group`/`exit_group`,
  `end_of_scope_msg`, `mode_stack`, `group_depth`. `record_span` now returns
  `syn::Result<()>`.
  - **Migration**: fatality lives on the error (`ParseError::is_fatal`), no
    longer on the context. Scopes live under `ctx.scopes`.

- **All combinators from `grammar-kit` are removed without replacement**:
  `attempt`, `peek`, `not_check`, `attempt_recover`, `parse_ident`,
  `parse_int`, `skip_until`. They took a `ParseStream`.
  - **Migration**: the counterparts are `peek_syn`, `finish_variants`,
    `parse_separated`, `parse_repeated`, and the new module `stream` (`Strom`,
    `StreamResult`, `parse_syn`, `parse_mit`, `gabel`, `uebernehmen`, `gruppe`,
    `schritt`, `token_nehmen`).

- **All `builtins::parse_*_impl` have a new signature**: instead of
  `<T: CommonBuiltins>(&mut T, &mut ParseContext) -> syn::Result<X>` now
  `<'a>(&rt::Strom<'a>, &mut ParseContext<'a>) -> StreamResult<'a, X>`. The
  trait `CommonBuiltins` and its `impl for ParseStream` are deleted.
  `token_filter::{alpha, alphanumeric, digit, hex_digit, oct_digit}` remain
  cursor primitives (`Cursor<'a> -> ParseResult<'a, X>`) and run on the stream
  via `rt::schritt`.

- **The generated `parse_X_impl` takes a `&rt::Strom<'a>`** (that is, a
  `&syn::parse::ParseBuffer<'a>`) and returns `rt::StreamResult<'a, T>`, i.e.
  `Result<T, ParseError<'a>>` with no cursor on success.
  - **Impact**: this is the hook for hand-written parsers (`extern` rules);
    their signature changes accordingly. The public wrapper
    `parse_X(ParseStream) -> syn::Result<T>` is unchanged - **end users are not
    affected**.
  - Deliberately `&ParseBuffer<'a>` and not syn's alias
    `ParseStream<'a> = &'a ParseBuffer<'a>`: the alias would shorten `'a` to
    the stack frame on a fork, so errors from a fork could no longer leave the
    call.
  - **Backtracking now goes through `rt::gabel`/`rt::uebernehmen`**
    (`fork`/`advance_to`) instead of a cursor copy. After an error the stream
    may have advanced; anyone who wants to backtrack must work on a fork. The
    code generator does so at every backtracking point.

- **`syn_grammar::Identifier` and `syn_grammar::StringLiteral` as root
  re-exports are gone.**
  - **Migration**: `syn_grammar::types::Identifier` and `::types::StringLiteral`.

- **The `include_grammar` macro is deleted.** Since 0.2.0 it had only emitted an
  explanatory `compile_error!`; now the compiler reports "cannot find macro".
  Grammars belong inline in `grammar! { … }`.

- **`syn_grammar_model::parse_grammar_with_builtins` no longer exists.**
  - **Migration**: `parse_grammar::<B: Backend>`. `model` no longer re-exports
    `backend::*`/`types::*` flatly.

- **Data model**: `GrammarDefinition` loses `inherits` and gains
  `extern_rules`/`imports`; `Rule` gains `return_type_kind` and `is_lexical`;
  `RuleCall.rule_name` is now `rule_path`; `params` is `Vec<RuleParameter>`.

- **Uppercase rule names are automatically lexical.** No whitespace is allowed
  between the patterns of such a rule any more.
  - **Impact**: an existing grammar with an uppercase rule changes behaviour
    *silently*. Either rename it or wrap it in `spaced(...)`.

- **`separated(..., trailing=false)`** no longer fails hard on a separator with
  no following item; it softly backtracks to before the separator.
  - **Impact**: grammars of the form `paren(list? ","?)` now work; grammars
    that relied on the hard error now accept silently.

- **`any_ident` now accepts keywords** (`Ident::parse_any`) and is therefore
  no longer equivalent to `ident`.
  - **Impact**: anyone who relied on `any_ident` failing on a keyword (say, to
    delimit an alternative chain) must switch to `ident`.

- **Error selection compares progress first**, only then fatality and
  priority.
  - **Impact**: an early `fail("…")` loses against a normal error that got
    further - whoever consumed more tokens was closer to the intended
    derivation. At the *same* position `fail` still wins. (An earlier version
    of this draft said "`fail` takes precedence" without qualification; that
    never applied to the progress comparison.)

- **The format of the rule context has changed**: instead of the prefix
  `Error in rule 'inner': …` there are now suffix lines `\nin inner rule`, with
  underscores as spaces, plus ` at column N (line M)` when the span carries
  position data.

- **Many fixed message texts have changed**, among them
  `No matching rule variant found` -> ``expected one of: `a`, `b`; found unexpected token `x` ``,
  `expected at least N items` -> `expected at least N <item>s, found M`,
  `unexpected match` -> ``unexpected match for rule `X`; found `Y` in rule `Z` ``.
  - **Migration**: the binding catalogue is now
    `docs/adr/adr13-error-message-contract.md`.

- **`grammar-kit::testing::TestResult<T, E>` has a third parameter
  `S = ()`**; the `'static` bound on `E` is gone;
  `assert_failure_contains`/`assert_failure_not_contains` return `Self` instead
  of `()`; `Testable` is generalised from `syn::Result<T>` to `Result<T, E>`.

- **`grammar-kit`: the features `rt` and `trace` are removed** - they switched
  nothing on. Likewise the never-working macro `test_both_backends!` and the
  never-included module `transaction`.

### Added

- **Linear parsing (ADR 15).** No parse step materialises the remaining
  stream any more. A `syn::Type` costs `input.parse::<T>()` instead of a new
  `TokenBuffer` over the entire rest of the enclosing delimiter group. Measured
  on an argument list with 2000 entries: 1.174 s -> 5.33 ms, and quadratic
  became linear (twenty times the input used to cost 356x, now 16x). Single
  tokens go through `take_single` in O(1), `peek_syn` without any allocation.

- **Cursor-based diagnostics engine.** Progress is compared via
  `syn::buffer::Cursor` (`PartialOrd`, O(1)) instead of line/column. Reason:
  the cursor comparison is a pointer comparison in O(1) and does not depend on
  any compiler version. (Up to Rust 1.87 `Span::start()` inside a procedural
  macro also returned `(0,0)` for *every* span; since 1.88 it no longer does.)
- **High-water mark for hidden errors** (`ParseContext::furthest`). An error
  that a *successful* backtrack covers up (`?`, `*`, `separated(min=0)`)
  survives and is reported when otherwise only a meaningless message would be
  left.
- **Live rule stack** (`enter_rule`/`exit_rule`) with a snapshot, so that a
  remembered error carries its rule context too.
- **`item_label=`** for `separated`/`repeated`: names list items and counts
  them (`expected function argument … in function argument 2`).
- **Separation of fatality and priority**: `ParseError::is_fatal` for the cut,
  `priority` for `fail(..)` and labels.
- **Distinction `unexpected end of group` / `unexpected end of input`** via
  the group depth in the context.
- **`extern` rules and `import`** in the grammar DSL.
- **`lex(...)` and `spaced(...)`** for explicit control of whitespace
  sensitivity, plus the uppercase convention.
- **`count(pattern)`**, which returns the number of matches.
- **Simplified rule syntax**: without the `rule` keyword, without a return
  type, without an action block.
- **New built-ins**: `any_ident`, `named_field`, `unnamed_field`,
  `visibility`, `generics`, `return_type`, `statements`, plus `pat`,
  `inner_attrs` and `lit_byte`. `pat` closes the biggest gap: `syn::Pat` has no
  `impl Parse` and was unreachable through the `syn::` path.
- **The `with_span` derive macro** and the `WithSpan` trait.
- **`DEBUG_GRAMMAR`**: an environment variable that prints the generated code
  to stderr.
- **`cxx-parser`** as the acceptance benchmark (its own crate, version 0.1.0).

### Fixed

- **`digit`, `hex_digit`, `oct_digit`** were declared as `syn::Ident` in the
  built-in catalogue but return `syn::LitInt`. The declared type drives
  generics inference - a generic rule instantiated with `digit` produced a
  compiler error in the *generated* code.
- **The "Undefined rule" check** was switched off by *any* `use` statement. It
  now hangs on the glob import (`use …::*;`), the only thing that can bring in
  unknown rule names. A typo in a rule name is reported cleanly again instead
  of as a follow-up error in generated code.
- **Discarded alternative errors were lost** when a later alternative
  succeeded and left input behind. Instead of a message like
  `expected integer literal … in term … in expression` only syn's
  `unexpected token` appeared.
- **A `syn::` type without `Parse`** (e.g. `syn::Field`) produced a raw
  trait-bound error on generated code. Now a message appears on the user's
  line, naming the built-in to use instead.
- **`peek_syn` and the bridge combinators** failed as soon as tokens followed
  the call - `Parser::parse2` requires the whole stream to be consumed. The
  bridge is gone entirely with ADR 15 stage 3.
- **Zero-progress guard** in the repetition loops: `*`, `+` and the list
  combinators could loop forever on a pattern that consumes nothing.
- **`LexicalScope`/`SpacedScope`** did not clean up the `mode_stack` on error
  (`?` came before `exit_mode()`).
- **The CI doc step** was red: `syn::parse::<Token>()` without backticks in a
  doc comment made rustdoc abort under `-D warnings`.

### Documentation

- `docs/ERROR_HANDLING.md` rewritten - it described the old engine (position
  comparison, priorities 0/1/2, message length as tie-break) and a label syntax
  that never existed.
- `docs/adr/adr13-error-message-contract.md` new: the binding catalogue for
  error messages, every point with its test location.
- `GOALS.md` and `ARCHITECTURE.md` new; `EXTENDING.md` and `docs/adr/adr1.md`
  marked as outdated and withdrawn respectively.
- `SYNTAX.md`: the label operator `#`, `item_label`, the `spanned_*` family and
  the span binding `@` were undocumented.
- `#![warn(missing_docs)]` in `grammar-kit` and `syn-grammar`; 58 missing doc
  comments added.

### Earlier entries of this draft

The 0.9.0 draft had been started before the rebuild described above. These
entries come from the first half and still apply - unless the text above says
otherwise.

#### Added
- **Consolidated Error Messages with Labeled Alternatives**: The parser can now produce a single, clear error message when multiple alternatives fail at the same position (e.g., "expected one of: an expression, a statement"). This is enabled by a new labeling mechanism, which uses rule names as default labels and supports explicit labels via the `# "label"` syntax. This replaces ambiguous, single-alternative errors with a helpful summary of all valid possibilities.
- **High-Priority Manual Error Reporting**: Added the `fail("message")` built-in rule, which always fails with a custom error message. Errors generated by `fail` carry a high priority and win against other errors **at the same position**, allowing grammar authors to provide precise, context-aware messages instead of the parser's default "expected" text. They do not, however, override an error that got *further* into the input - see the error-selection entry above.
- **Implicit Token Literals and Aliases**: The grammar parser now supports `char` literals (e.g., `'+'`) as a shorthand for single-token string literals. It also includes a prelude of common token aliases (e.g., `PLUS` for `"+"`).
- **Parametric List Rules (ADR 004)**: Added `separated` and `repeated` built-in rules for concise list parsing.
    - `separated(rule, sep, min=0, trailing=false)`: Parses a list of items separated by a delimiter.
    - `repeated(rule, min=0)`: Parses a list of items without a separator.
    - Supports custom container types via generics (e.g., `separated<HashSet>(...)`), defaulting to `Vec`.
- **Named Arguments**: Added support for named arguments in rule calls (e.g., `rule(key=value)`), used by the new list rules.
- **Generic Arguments in Rules**: Added support for generic type arguments in rule calls (e.g., `rule<T>(...)`), enabling the container specification for list rules.
- **Until**: Added support for the `until` pattern (e.g., `body:until(";")`), which consumes tokens until a terminator pattern is found. The terminator is not consumed. This is useful for parsing unstructured content or content with a known delimiter.
- **Group Bindings**: Added support for binding a group of patterns to a variable (e.g. `var:("a" | "b")`). This captures the result of the group (which matches the inner pattern).

#### Fixed
- **Built-in Rule Arguments**: Fixed an issue where the validator incorrectly forbade arguments for built-in rules. This allows backend-specific built-ins (or future portable built-ins) to accept arguments as needed.

#### Breaking Changes
- **Removed `<_>` Disambiguation**: Removed the `<_>` hack for rule arguments and generic calls. Instead, rule calls with arguments now require named arguments (e.g., `rule(name=arg)`) or template parameters to disambiguate them from EBNF groupings. Built-ins like `fail`, `peek`, `not` as well as `separated` and `repeated` are unaffected and can be used without named arguments.

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
