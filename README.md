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
- **Testing Utilities**: Fluent API for testing your parsers with pretty-printed error reporting.

## Installation

### 1. Quick Installation (Runtime Parsing)

Use this setup if you want to parse strings **at runtime** inside your application. This is the standard approach for:
- **CLIs & Interpreters**: Parsing user input or commands.
- **Configuration Files**: Reading custom config formats at startup.
- **Prototyping**: Experimenting with grammars in `main.rs`.

Add `syn-grammar` and `syn` to your `Cargo.toml`. `syn` is required at runtime because the generated parser relies on its types (e.g., `ParseStream`, `Ident`).

```toml
[dependencies]
syn-grammar = "0.4"
syn = { version = "2.0", features = ["full", "extra-traits"] }
quote = "1.0"
proc-macro2 = "1.0"
```

### 2. Optimized Installation (Compile-Time Macros)

If you are writing a **procedural macro** to parse input **at compile time**, you should isolate your parser definition in a separate crate. This is the correct approach for:

- **Embedded DSLs**: Parsing custom syntax inside Rust code (e.g., HTML-like templates, State Machines, SQL-like queries).
- **Code Generation**: Reading an external definition file during the build and generating Rust code from it.
- **Compile-Time Verification**: Checking syntax or configuration validity during `cargo build`.

**Steps:**

1. Create a separate `proc-macro` crate.
2. Add `syn-grammar`, `syn`, and `quote` to **that** crate's `Cargo.toml`.
3. Define your grammar and macro there.
4. Depend on that crate from your main project.

**Why?** Your main project will use the macro to generate code, but the heavy `syn` parsing logic will not be compiled into your final binary. This significantly improves build times for users of your macro.

### ⚠️ Important Note on Tokenization

Since `syn-grammar` is built on top of `syn`, it uses the **Rust Tokenizer**. This means your grammar must consist of valid Rust tokens.

- **Good Use Cases**: Grammars that look somewhat like code or data structures (e.g., JSON, mathematical expressions, C-like syntax, HTML tags).
- **Limitations**: You cannot parse languages that require a custom lexer, such as:
    - **Whitespace-sensitive languages** (e.g., Python, YAML) — `syn` skips whitespace automatically.
    - **Binary formats**.
    - **Arbitrary text** that doesn't form valid Rust tokens (e.g., unquoted strings with special characters like `@` or `$` in positions Rust doesn't allow).

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

```text
rule name -> ReturnType = pattern -> { action_code }
```

- **`name`**: The name of the rule (e.g., `expr`).
- **`ReturnType`**: The Rust type returned by the rule (e.g., `Expr`, `i32`, `Vec<String>`).
- **`pattern`**: The EBNF pattern defining what to parse.
- **`action_code`**: A Rust block that constructs the return value from the bound variables.

#### Attributes and Doc Comments

Rules can be decorated with standard Rust attributes and documentation comments. These are passed through to the generated function.

```rust
use syn_grammar::grammar;
use syn::Ident;

grammar! {
    grammar MyGrammar {
        /// Parses a valid identifier.
        #[cfg(feature = "extra")]
        rule my_ident -> Ident = i:ident -> { i }
    }
}

```

### Rule Arguments

Rules can accept arguments, allowing you to pass context or state down the parser chain.

```rust
use syn_grammar::grammar;

grammar! {
    grammar Args {
        rule main -> i32 = 
            "start" v:value(10) -> { v }

        rule value(offset: i32) -> i32 =
            i:integer -> { i + offset }
    }
}
```

### Grammar Inheritance

You can inherit rules from another grammar module. This is useful for splitting large grammars or reusing common rules.

```rust
mod base {
    use syn_grammar::grammar;
    grammar! {
        grammar Base {
            pub rule num -> i32 = i:integer -> { i }
        }
    }
}

use syn_grammar::grammar;
use base::Base;

grammar! {
    grammar Derived : Base {
        rule main -> i32 = 
            "add" a:num b:num -> { a + b }
    }
}
# fn main() {}
```

### Patterns

#### Literals and Keywords
Match specific tokens using string literals.

```rust
use syn_grammar::grammar;

grammar! {
    grammar Kws {
        rule kw -> () = "fn" "name" -> { () }
    }
}
```

#### Multi-token Literals
You can match sequences of tokens that must appear strictly adjacent to each other (no whitespace) by using a single string literal containing multiple tokens.

```rust
use syn_grammar::grammar;

grammar! {
    grammar Tokens {
        // Matches "?." (e.g. in `foo?.bar`)
        // Fails if there is a space like `? .`
        rule optional_dot -> () = "?." -> { () }

        // Matches "@detached" (Punct `@` + Ident `detached`) without space
        rule attribute -> () = "@detached" -> { () }
    }
}
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

```rust
use syn_grammar::grammar;
use syn::Ident;

// Mock Stmt for the example
pub enum Stmt {
    Assign(Ident, i32),
}

grammar! {
    grammar Assignment {
        rule assignment -> super::Stmt = 
            name:ident "=" val:expr -> { 
                super::Stmt::Assign(name, val) 
            }
            
        rule expr -> i32 = i:integer -> { i }
    }
}
# fn main() {}
```

#### Alternatives (`|`)
Match one of several alternatives. The first one that matches wins.

```rust
use syn_grammar::grammar;

grammar! {
    grammar Choice {
        rule choice -> bool = 
            "yes" -> { true }
          | "no"  -> { false }
    }
}
```

#### Repetitions (`*`, `+`, `?`)
- `pattern*`: Match zero or more times. Returns a `Vec`.
- `pattern+`: Match one or more times. Returns a `Vec`.
- `pattern?`: Match zero or one time. Returns an `Option` (or `()` if unbound).

```rust
use syn_grammar::grammar;

grammar! {
    grammar List {
        rule list -> Vec<i32> = 
            [ elements:integer* ] -> { elements }
    }
}
```

#### Groups `(...)`
Group patterns together to apply repetitions or ensure precedence.

```rust
use syn_grammar::grammar;

grammar! {
    grammar Group {
        rule complex -> () = 
            ("a" | "b")+ "c" -> { () }
    }
}
```

#### Delimiters
Match content inside delimiters.

**Note**: You cannot match delimiters using string literals (e.g., `"["` or `"}"`) because `syn` parses them as structural `TokenTree`s. You must use the syntax below.

- `paren(pattern)`: Matches `( pattern )`.
- `[ pattern ]`: Matches `[ pattern ]`.
- `{ pattern }`: Matches `{ pattern }`.

```rust
use syn_grammar::grammar;

grammar! {
    grammar Tuple {
        rule tuple -> (i32, i32) = 
            paren(a:integer "," b:integer) -> { (a, b) }
    }
}
```

#### Error Recovery (`recover`)
You can make your parser robust against errors using `recover(rule, sync_token)`.
If `rule` fails, the parser will skip tokens until it finds `sync_token`, returning `None` (or `(None, ...)` for bindings).
Note that `recover` does **not** consume the sync token.

```rust
use syn_grammar::grammar;

#[derive(Debug)]
pub struct Stmt;

grammar! {
    grammar Recovery {
        rule stmt -> Option<super::Stmt> =
            // If `parse_stmt` fails, skip until `;`
            // `s` will be `Option<Stmt>` (Some if success, None if recovered)
            s:recover(parse_stmt, ";") ";" -> { s }
            
        rule parse_stmt -> super::Stmt = "let" "x" -> { super::Stmt }
    }
}
# fn main() {}
```

### The Cut Operator (`=>`)

The cut operator `=>` allows you to commit to a specific alternative. If the pattern *before* the `=>` matches, the parser will **not** backtrack to try other alternatives, even if the pattern *after* the `=>` fails. This produces better error messages.

```rust
use syn_grammar::grammar;
use syn::Ident;

pub enum Stmt {
    Let(Ident, i32),
    Expr(i32),
}

grammar! {
    grammar Cut {
        rule stmt -> super::Stmt =
            // If we see "let", we commit to this rule. 
            // If "mut" or the identifier is missing, we error immediately 
            // instead of trying the next alternative.
            "let" => "mut"? name:ident "=" e:expr -> { super::Stmt::Let(name, e) }
          | e:expr -> { super::Stmt::Expr(e) }
          
        rule expr -> i32 = i:integer -> { i }
    }
}
# fn main() {}
```

## Testing

`syn-grammar` provides a fluent testing API via the `grammar-kit` crate (re-exported as `syn_grammar::testing`). When tests fail, errors are pretty-printed with source context and underlining.

```rust,no_run
use syn_grammar::grammar;
use syn_grammar::testing::Testable;

grammar! {
    grammar Calc {
        rule expression -> i32 = 
            l:expression "+" r:term -> { l + r }
          | t:term -> { t }
        
        rule term -> i32 = i:integer -> { i }
    }
}

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
# fn main() {}
```

## Advanced Topics

### Left Recursion

Recursive descent parsers typically struggle with left recursion (e.g., `A -> A b`). `syn-grammar` automatically detects direct left recursion and compiles it into an iterative loop. This makes writing expression parsers natural and straightforward.

```rust
use syn_grammar::grammar;

grammar! {
    grammar Expr {
        // This works perfectly!
        rule expr -> i32 = 
            l:expr "+" r:term -> { l + r }
          | t:term            -> { t }
          
        rule term -> i32 = i:integer -> { i }
    }
}
```

### Backtracking

By default, `syn-grammar` uses `syn`'s speculative parsing (`fork`) to try alternatives.
1. It checks if the next token matches the start of an alternative (using `peek`).
2. If ambiguous, it attempts to parse the alternative.
3. If it fails, it backtracks and tries the next one.

This allows for flexible grammars but can impact performance if overused. Use the **Cut Operator** (`=>`) to prune the search space when possible.

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
