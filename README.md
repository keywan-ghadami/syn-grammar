# syn-grammar

[![Crates.io](https://img.shields.io/crates/v/syn-grammar.svg)](https://crates.io/crates/syn-grammar)
[![Documentation](https://docs.rs/syn-grammar/badge.svg)](https://docs.rs/syn-grammar)
[![License](https://img.shields.io/crates/l/syn-grammar.svg)](https://github.com/keywan-ghadami/syn-grammar/blob/main/LICENSE)

**syn-grammar** is a parser generator for Rust that allows you to define EBNF-like grammars directly inside your code and compiles them into `syn` parsers.

It is designed to make writing procedural macros and Domain Specific Languages (DSLs) in Rust significantly easier by handling the parsing boilerplate for you.

## Features

- **EBNF Syntax**: Define rules using sequences, alternatives (`|`), optionals (`?`), repetitions (`*`, `+`), and groups `(...)`.
- **Type-Safe Actions**: Attach Rust code blocks (`-> { ... }`) to rules to transform parsed tokens into your own AST or `syn` structures.
- **Syn Integration**: Built-in support for parsing Rust identifiers (`ident`), integers (`int_lit`), and strings (`string_lit`).
- **Left Recursion**: Automatically handles direct left recursion (e.g., `expr = expr "+" term`), making expression parsing intuitive.
- **Backtracking**: Supports speculative parsing for ambiguous grammars.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
syn-grammar = "0.1"
syn = { version = "2.0", features = ["full", "extra-traits"] }
quote = "1.0"
proc-macro2 = "1.0"
```

## Quick Start

### Inline Grammar

You can define a grammar directly inside a macro:

```rust
use syn_grammar::grammar;
use syn::parse::Parser; // Required for .parse_str()

grammar! {
    grammar Calc {
        // The return type of the rule is defined after `->`
        rule expression -> i32 =
            l:expression "+" r:term -> { l + r }
          | t:term                  -> { t }

        rule term -> i32 =
            f:factor "*" t:term -> { f * t }
          | f:factor            -> { f }

        rule factor -> i32 =
            i:int_lit      -> { i }
          | paren(e:expression) -> { e }
    }
}

fn main() {
    // The macro generates a module `Calc` with functions `parse_<rule_name>`
    let result = Calc::parse_expression.parse_str("1 + 2 * 3");
    assert_eq!(result.unwrap(), 7);
}
```

## Syntax Reference

| Syntax | Description | Example |
|--------|-------------|---------|
| `"lit"` | Literal match | `"fn"` |
| `ident` | Rust Identifier | `my_var` |
| `int_lit` | Integer Literal | `42` |
| `string_lit` | String Literal | `"hello"` |
| `name:rule` | Rule call with binding | `e:expr` |
| `( A B )` | Grouping | `("a" "b")` |
| `A \| B` | Alternatives | `"true" \| "false"` |
| `A?` | Optional | `","?` |
| `A*` | Zero or more | `item*` |
| `A+` | One or more | `digit+` |
| `paren(A)` | Parentheses `(...)` | `paren(expr)` |
| `bracketed[A]` | Brackets `[...]` | `bracketed[expr]` |
| `braced{A}` | Braces `{...}` | `braced{expr}` |
| `A => B` | Cut Operator (Commit) | `"let" => "mut"` |

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
