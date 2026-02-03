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
- **Rule Arguments**: Pass context or parameters between rules.
- **Grammar Inheritance**: Reuse rules from other grammars.
- **Testing Utilities**: Fluent API for testing your parsers.

## Installation

Add `syn-grammar` to your `Cargo.toml`.

You also need to add `syn`, as the generated code relies on its types (e.g., `ParseStream`). If you are writing a procedural macro, you will likely need `quote` and `proc-macro2` as well.

```toml
[dependencies]
syn-grammar = "0.3"
syn = { version = "2.0", features = ["full", "extra-traits"] }
quote = "1.0"
proc-macro2 = "1.0"
```

### Managing Dependencies

Since `syn` and `quote` are heavy dependencies, it is recommended to isolate your parser definition in a separate crate.

If you are writing a **procedural macro**:
1. Create a separate `proc-macro` crate for your macro definition.
2. Add `syn-grammar`, `syn`, and `quote` to that crate's `Cargo.toml`.
3. Define your grammar and macro there.
4. Depend on that crate from your main project.

Your main project will use the macro but will **not** need to compile `syn` or `syn-grammar` itself, significantly improving build times.

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
            i:integer           -> { i }
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

#### Attributes and Doc Comments

Rules can be decorated with standard Rust attributes and documentation comments. These are passed through to the generated function.

```rust,ignore
/// Parses a valid identifier.
#[cfg(feature = "extra")]
rule ident -> Ident = ...
```

### Rule Arguments

Rules can accept arguments, allowing you to pass context or state down the parser chain.

```rust,ignore
rule main -> i32 = 
    "start" v:value(10) -> { v }

rule value(offset: i32) -> i32 =
    i:integer -> { i + offset }
```

### Grammar Inheritance

You can inherit rules from another grammar module. This is useful for splitting large grammars or reusing common rules.

```rust,ignore
// In base.rs
grammar! {
    grammar Base {
        pub rule num -> i32 = i:integer -> { i }
    }
}

// In derived.rs
use crate::base::Base;

grammar! {
    grammar Derived : Base {
        rule main -> i32 = 
            "add" a:num b:num -> { a + b }
    }
}
```

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
| `integer` | An integer literal (e.g., `42`) | `i32` |
| `string` | A string literal (e.g., `"hello"`) | `String` |
| `lit_str` | A string literal object | `syn::LitStr` |
| `rust_type` | A Rust type (e.g., `Vec<i32>`) | `syn::Type` |
| `rust_block` | A block of code (e.g., `{ stmt; }`) | `syn::Block` |
| `lit_int` | A typed integer literal (e.g. `1u8`) | `syn::LitInt` |
| `lit_char` | A character literal (e.g. `'c'`) | `syn::LitChar` |
| `lit_bool` | A boolean literal (`true` or `false`) | `syn::LitBool` |
| `lit_float` | A floating point literal (e.g. `3.14`) | `syn::LitFloat` |
| `spanned_int_lit` | An integer literal with span | `(i32, Span)` |
| `spanned_string_lit` | A string literal with span | `(String, Span)` |
| `spanned_float_lit` | A float literal with span | `(f64, Span)` |
| `spanned_bool_lit` | A bool literal with span | `(bool, Span)` |
| `spanned_char_lit` | A char literal with span | `(char, Span)` |
| `outer_attrs` | Outer attributes (e.g. `#[...]`) | `Vec<syn::Attribute>` |

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
    "[" elements:integer* "]" -> { elements }
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
    paren(a:integer "," b:integer) -> { (a, b) }
```

#### Error Recovery (`recover`)
You can make your parser robust against errors using `recover(rule, sync_token)`.
If `rule` fails, the parser will skip tokens until it finds `sync_token`, returning `None` (or `(None, ...)` for bindings).
Note that `recover` does **not** consume the sync token.

```rust,ignore
rule stmt -> Option<Stmt> =
    // If `parse_stmt` fails, skip until `;`
    // `s` will be `Option<Stmt>` (Some if success, None if recovered)
    s:recover(parse_stmt, ";") ";" -> { s }
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

## Testing

`syn-grammar` provides a fluent testing API via the `grammar-kit` crate (re-exported as `syn_grammar::testing`).

```rust,ignore
use syn_grammar::testing::Testable;

#[test]
fn test_calc() {
    Calc::parse_expression
        .parse_str("1 + 2")
        .test()
        .assert_success_is(3);

    Calc::parse_expression
        .parse_str("1 + *")
        .test()
        .assert_failure_contains("expected term");
}
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
