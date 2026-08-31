# syn-grammar

[![Crates.io](https://img.shields.io/crates/v/syn-grammar.svg)](https://crates.io/crates/syn-grammar)
[![Documentation](https://docs.rs/syn-grammar/badge.svg)](https://docs.rs/syn-grammar)
[![License](https://img.shields.io/crates/l/syn-grammar.svg)](https://github.com/keywan-ghadami/syn-grammar/blob/main/LICENSE)

**syn-grammar** is a powerful parser generator for Rust that allows you to define EBNF-like grammars directly inside your code. It compiles these definitions into efficient `syn` parsers at compile time.

Writing parsers for procedural macros or Domain Specific Languages (DSLs) in Rust often involves writing repetitive boilerplate code using the `syn` crate. **syn-grammar** simplifies this process by letting you describe *what* you want to parse using a clean, readable syntax.

## Documentation

- **[Grammar Syntax Reference](SYNTAX.md)**: Detailed guide to the shared grammar definition language (rules, operators, built-ins).
- **[Extending Guide](../EXTENDING.md)**: Guide for library authors on how to build custom backends.

## Features

- **Inline Grammars**: Define your grammar directly in your Rust code using the `grammar!` macro.
- **Black-Box Grammar Composition**: Safely compose grammars across modules and crates using `import` and `extern` interfaces.
- **Type-Safe Actions**: Directly map parsing rules to Rust types and AST nodes using action blocks (`-> { ... }`).
- **Seamless Syn Integration**: First-class support for parsing Rust tokens like identifiers, literals, types, and blocks.
- **Testing Utilities**: Fluent API for testing your parsers with pretty-printed error reporting.

## Installation

### 1. Quick Installation (Runtime Parsing)

Use this setup if you want to parse strings **at runtime** inside your application. This is the standard approach for CLIs, interpreters, or configuration files.

```toml
[dependencies]
syn-grammar = "0.8.0"
syn = { version = "2.0", features = ["full", "extra-traits"] }
quote = "1.0"
proc-macro2 = "1.0"
```

### 2. Optimized Installation (Compile-Time Macros)

If you are writing a **procedural macro** to parse input **at compile time**, you should isolate your parser definition in a separate crate. This significantly improves build times.

1. Create a separate `proc-macro` crate.
2. Add `syn-grammar`, `syn`, and `quote` to **that** crate's `Cargo.toml`.
3. Define your grammar and macro there.
4. Depend on that crate from your main project.

### ⚠️ Important Note on Tokenization

Since `syn-grammar` is built on top of `syn`, it uses the **Rust Tokenizer**. This means your grammar must consist of valid Rust tokens.

- **Good Use Cases**: Grammars that look somewhat like code or data structures (e.g., JSON, mathematical expressions, C-like syntax).
- **Limitations**: You cannot parse languages that require a custom lexer (e.g., whitespace-sensitive languages, binary formats).

## Quick Start

Here is a complete example of a calculator grammar that parses mathematical expressions, including parenthesized sub-expressions.

```rust
use syn_grammar::grammar;
use syn::parse::Parser; // Required for .parse_str()

grammar! {
    grammar Calc {
        pub rule expression -> i32 =
            l:expression "+" r:term -> { l + r }
          | l:expression "-" r:term -> { l - r }
          | t:term                  -> { t }

        rule term -> i32 =
            f:factor "*" t:term -> { f * t }
          | f:factor "/" t:term -> { f / t }
          | f:factor            -> { f }

        rule factor -> i32 =
            i:i32                     -> { i }
          | paren(e:expression) -> { e } // Matches literal ( ... ) in the input
    }
}

fn main() {
    // The macro generates a module `Calc` containing a function `parse_expression`
    // corresponding to the `expression` rule.
    let result = Calc::parse_expression.parse_str("10 - (2 * 3)");
    assert_eq!(result.unwrap(), 4);
}
```

### What happens under the hood?

The `grammar!` macro expands into a Rust module (named `Calc` in the example) containing:
- A function `parse_<rule_name>` for each rule (e.g., `parse_expression`).
- These functions take a `syn::parse::ParseStream` and return a `syn::Result<T>`.

## Backend Specifics

### Input Type
The generated parser functions take a `syn::parse::ParseStream`.

### Built-ins
In addition to the portable built-ins (see [SYNTAX.md](SYNTAX.md)), `syn-grammar` provides the following `syn`-specific parsers:

| Parser | Description | Returns |
|---|---|---|
| `rust_type` | A Rust type (e.g., `Vec<i32>`) | `syn::Type` |
| `rust_block` | A block of code (e.g., `{ stmt; }`) | `syn::Block` |
| `lit_str` | A string literal object | `syn::LitStr` |
| `lit_int` | A typed integer literal (e.g. `1u8`) | `syn::LitInt` |
| `lit_char` | A character literal | `syn::LitChar` |
| `lit_bool` | `true` or `false` | `syn::LitBool` |
| `lit_float` | A float literal | `syn::LitFloat` |
| `lit_byte` / `any_byte` | A byte literal (`b'A'`) | `syn::LitByte` |
| `outer_attrs` | Outer attributes (`#[...]`) | `Vec<syn::Attribute>` |
| `inner_attrs` | Inner attributes (`#![...]`) | `Vec<syn::Attribute>` |
| `any_ident` | An identifier, **keywords included** (`Ident::parse_any`) | `syn::Ident` |
| `pat` | A Rust pattern (`Some(x)`, `A \| B`) | `syn::Pat` |
| `visibility` | A visibility (`pub`, `pub(crate)`) | `syn::Visibility` |
| `generics` | A generic parameter list (`<T, U>`) | `syn::Generics` |
| `return_type` | A return type (`-> i32`) | `syn::ReturnType` |
| `named_field` | A named struct field (`name: i32`) | `syn::Field` |
| `unnamed_field` | A tuple-struct field (`i32`) | `syn::Field` |
| `statements` | A sequence of statements | `Vec<syn::Stmt>` |

Any other `syn` type that implements `syn::parse::Parse` can be used directly by
writing its path — `x:syn::Expr`, `t:syn::Type`, `p:syn::Path`. A `syn` type
*without* `Parse` (such as `syn::Field`) produces a grammar-level error naming
the built-in to use instead.

Beyond these there is a `spanned_` variant for every numeric and character
primitive (`spanned_i32`, `spanned_u64`, `spanned_f32`, `spanned_char`,
`spanned_bool`, …) returning `syn_grammar::types::SpannedValue<T>` with both
`value` and `span`. See [SYNTAX.md](SYNTAX.md).

### Return Types
Portable built-ins map to specific `syn` or `syn-grammar` types:

| Portable Primitive | Return Type | Notes |
|---|---|---|
| `ident` | `syn_grammar::types::Identifier` | Wraps `syn::Ident`, implements `PartialEq`, `Hash`. |
| `string` | `syn_grammar::types::StringLiteral` | Wraps `syn::LitStr`. |
| `alpha`, `alphanumeric` | `syn::Ident` | |
| `digit` | `syn::LitInt` | |

### Span Binding (`@`)
You can capture the `Span` of a parsed rule or built-in using `name:rule @ span_var`. The rule must return a type that implements `syn::spanned::Spanned` (e.g., `syn::Ident`, `syn::Type`, `syn_grammar::types::Identifier`).

## Testing

`syn-grammar` provides a fluent testing API via the `grammar-kit` crate (re-exported as `syn_grammar::testing`).

```rust
use syn::parse::Parser;
use syn_grammar::grammar;
use syn_grammar::testing::Testable;

grammar! {
    grammar Calc {
        pub rule expression -> i32 =
            l:expression "+" r:term -> { l + r }
          | t:term -> { t }

        rule term -> i32 = i:i32 -> { i }
    }
}

fn main() {
    Calc::parse_expression
        .parse_str("1 + 2")
        .test()
        .assert_success_is(3);

    // Der Fehler benennt die Erwartung und den Weg dorthin:
    //
    //     expected integer literal at column 4 (line 1)
    //     in term
    //     in expression
    Calc::parse_expression
        .parse_str("1 + *")
        .test()
        .assert_failure_contains("expected integer literal")
        .assert_failure_contains("in term")
        .assert_failure_contains("in expression");
}
```

## Contributing

To contribute to `syn-grammar`, please ensure high quality by following these steps before committing:

1.  **Format Code**: Run `cargo fmt` to ensure consistent style.
2.  **Lint Code**: Run `cargo clippy --workspace --all-targets -- -D warnings` to catch common mistakes and enforce best practices.
3.  **Run Tests**: Run `cargo test --workspace` to ensure all functionality works as expected.

Die genauen Befehle stehen in `.github/workflows/ci.yaml`.

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
