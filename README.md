# syn-grammar

[![Crates.io](https://img.shields.io/crates/v/syn-grammar.svg)](https://crates.io/crates/syn-grammar)
[![Documentation](https://docs.rs/syn-grammar/badge.svg)](https://docs.rs/syn-grammar)
[![License](https://img.shields.io/crates/l/syn-grammar.svg)](https://github.com/keywan-ghadami/syn-grammar/blob/main/LICENSE)

**syn-grammar** is a powerful parser generator for Rust that allows you to define EBNF-like grammars directly inside your code. It compiles these definitions into efficient `syn` parsers at compile time.

Writing parsers for procedural macros or Domain Specific Languages (DSLs) in Rust often involves writing repetitive boilerplate code using the `syn` crate. **syn-grammar** simplifies this process by letting you describe *what* you want to parse using a clean, readable syntax, while handling the complex logic of parsing, backtracking, and error reporting for you.

## Features

- **Inline Grammars**: Define your grammar directly in your Rust code using the `grammar!` macro.
- **EBNF Syntax**: Familiar syntax with sequences, alternatives (`|`), optionals (`?`), repetitions (`*`, `+`), and grouping `(...)`.
- **Type-Safe Actions**: Directly map parsing rules to Rust types and AST nodes using action blocks (`-> { ... }`).
- **Seamless Syn Integration**: First-class support for parsing Rust tokens like identifiers, literals, types, and blocks.
- **Automatic Left Recursion**: Write natural expression grammars (e.g., `expr = expr + term`) without worrying about infinite recursion.
- **Backtracking & Ambiguity**: Automatically handles ambiguous grammars with speculative parsing.
- **Cut Operator**: Control backtracking explicitly for better error messages and performance.

## Installation

Add `syn-grammar` to your `Cargo.toml`. You will also likely need `syn`, `quote`, and `proc-macro2` as they are used in the generated code.

```toml
[dependencies]
syn-grammar = "0.1"
syn = { version = "2.0", features = ["full", "extra-traits"] }
quote = "1.0"
proc-macro2 = "1.0"
```

## Quick Start

Here is a complete example of a calculator grammar that parses mathematical expressions into an `i32`.

```rust
use syn_grammar::grammar;
use syn::parse::Parser; // Required for .parse_str()

grammar! {
    grammar Calc {
        // The return type of the rule is defined after `->`
        pub rule expression -> i32 =
            l:expression "+" r:term -> { l + r }
          | l:expression "-" r:term -> { l - r }
          | t:term                  -> { t }

        rule term -> i32 =
            f:factor "*" t:term -> { f * t }
          | f:factor "/" t:term -> { f / t }
          | f:factor            -> { f }

        rule factor -> i32 =
            i:int_lit           -> { i }
          | paren(e:expression) -> { e }
    }
}

fn main() {
    // The macro generates a module `Calc` containing a function `parse_expression`
    // corresponding to the `expression` rule.
    let result = Calc::parse_expression.parse_str("10 - 2 * 3");
    assert_eq!(result.unwrap(), 4);
}
```

### What happens under the hood?

The `grammar!` macro expands into a Rust module (named `Calc` in the example) containing:
- A function `parse_<rule_name>` for each rule (e.g., `parse_expression`).
- These functions take a `syn::parse::ParseStream` and return a `syn::Result<T>`.
- All necessary imports and helper functions to make the parser work.

## Detailed Syntax Guide

### Rules

A grammar consists of a set of rules. Each rule has a name, a return type, and a pattern to match.

```rust,ignore
rule name -> ReturnType = pattern -> { action_code }
```

- **`name`**: The name of the rule (e.g., `expr`).
- **`ReturnType`**: The Rust type returned by the rule (e.g., `Expr`, `i32`, `Vec<String>`).
- **`pattern`**: The EBNF pattern defining what to parse.
- **`action_code`**: A Rust block that constructs the return value from the bound variables.

### Patterns

#### Literals and Keywords
Match specific tokens using string literals.

```rust,ignore
rule kw -> () = "fn" "name" -> { () }
```

#### Built-in Parsers
`syn-grammar` provides several built-in parsers for common Rust tokens:

| Parser | Description | Returns |
|--------|-------------|---------|
| `ident` | A Rust identifier (e.g., `foo`, `_bar`) | `syn::Ident` |
| `int_lit` | An integer literal (e.g., `42`) | `i32` |
| `string_lit` | A string literal (e.g., `"hello"`) | `String` |
| `lit_str` | A string literal object | `syn::LitStr` |
| `rust_type` | A Rust type (e.g., `Vec<i32>`) | `syn::Type` |
| `rust_block` | A block of code (e.g., `{ stmt; }`) | `syn::Block` |

#### Sequences and Bindings
Match a sequence of patterns. Use `name:pattern` to bind the result to a variable available in the action block.

```rust,ignore
rule assignment -> Stmt = 
    name:ident "=" val:expr -> { 
        Stmt::Assign(name, val) 
    }
```

#### Alternatives (`|`)
Match one of several alternatives. The first one that matches wins.

```rust,ignore
rule boolean -> bool = 
    "true"  -> { true }
  | "false" -> { false }
```

#### Repetitions (`*`, `+`, `?`)
- `pattern*`: Match zero or more times. Returns a `Vec`.
- `pattern+`: Match one or more times. Returns a `Vec`.
- `pattern?`: Match zero or one time. Returns an `Option` (or `()` if unbound).

```rust,ignore
rule list -> Vec<i32> = 
    "[" elements:int_lit* "]" -> { elements }
```

#### Groups `(...)`
Group patterns together to apply repetitions or ensure precedence.

```rust,ignore
rule complex -> () = 
    ("a" | "b")+ "c" -> { () }
```

#### Delimiters
Match content inside delimiters.

- `paren(pattern)`: Matches `( pattern )`.
- `bracketed[pattern]`: Matches `[ pattern ]`.
- `braced{pattern}`: Matches `{ pattern }`.

```rust,ignore
rule tuple -> (i32, i32) = 
    paren(a:int_lit "," b:int_lit) -> { (a, b) }
```

### The Cut Operator (`=>`)

The cut operator `=>` allows you to commit to a specific alternative. If the pattern *before* the `=>` matches, the parser will **not** backtrack to try other alternatives, even if the pattern *after* the `=>` fails. This produces better error messages.

```rust,ignore
rule stmt -> Stmt =
    // If we see "let", we commit to this rule. 
    // If "mut" or the identifier is missing, we error immediately 
    // instead of trying the next alternative.
    "let" => "mut"? name:ident "=" e:expr -> { ... }
  | e:expr -> { ... }
```

## Advanced Topics

### Left Recursion

Recursive descent parsers typically struggle with left recursion (e.g., `A -> A b`). `syn-grammar` automatically detects direct left recursion and compiles it into an iterative loop. This makes writing expression parsers natural and straightforward.

```rust,ignore
// This works perfectly!
rule expr -> i32 = 
    l:expr "+" r:term -> { l + r }
  | t:term            -> { t }
```

### Backtracking

By default, `syn-grammar` uses `syn`'s speculative parsing (`fork`) to try alternatives.
1. It checks if the next token matches the start of an alternative (using `peek`).
2. If ambiguous, it attempts to parse the alternative.
3. If it fails, it backtracks and tries the next one.

This allows for flexible grammars but can impact performance if overused. Use the **Cut Operator** (`=>`) to prune the search space when possible.

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
