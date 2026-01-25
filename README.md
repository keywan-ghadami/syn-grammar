# syn-grammar

**A parser generator for Rust that compiles EBNF-like grammars into `syn::parse::Parse` implementations.**

`syn-grammar` bridges the gap between grammar-based parser generators (like `pest` or `lalrpop`) and hand-written recursive descent parsers using `syn`. It allows you to define your language syntax declaratively using the `grammar!` macro while embedding raw Rust code for the AST construction.

> **Note:** This project was originally built as the Stage 0 Bootstrap tool for the **Nikaia** programming language but has been extracted into a standalone library.

## Features

* **Syn-Powered:** Generates robust Rust code using `syn` and `quote`. The output depends only on `syn`, not on this library.
* **Grammar Inheritance:** Supports modular language definition. You can define a `Core` grammar and extend it in `Advanced` grammars (`grammar Child : Parent`).
* **Rust Actions:** Embed arbitrary Rust code in your grammar rules (`-> { ... }`) to construct your AST nodes directly.

## Usage

### 1. Add to `dependencies`
In your `Cargo.toml`:

```toml
[dependencies]
syn-grammar = "0.1"
```

### 2. Define your Grammar
You can define the grammar directly in your Rust code using the `grammar!` macro.

```rust
use syn_grammar::grammar;

grammar! {
    grammar Calc {
        // Top-level rule returning a Rust type (e.g., i32)
        pub rule expr -> i32 = 
            | "add" a:int_lit() b:int_lit() -> { a + b }
            | "sub" a:int_lit() b:int_lit() -> { a - b }
            | v:int_lit() -> { v }
            
        // 'int_lit' is a built-in that maps to syn::LitInt
    }
}

fn main() {
    // The macro generates a module 'Calc' with a parser function 'parse_expr'
    // You can use .parse_str(...) provided by syn::parse::Parser
    let result = Calc::parse_expr.parse_str("add 1 2");
    assert_eq!(result.unwrap(), 3);
}
```

## Syntax Reference

### Rules and Variants
Rules are defined as `rule name -> ReturnType = pattern -> { action }`.
Multiple variants can be separated by `|`.

```nika
rule item -> Item =
    | "fn" name:ident() -> { Item::Fn(name) }
    | "struct" name:ident() -> { Item::Struct(name) }
```

### Inheritance
Grammars can inherit from others to reuse or override rules. This is useful for layering complex syntax on top of a core definition.

```nika
grammar Extended : Core {
    // Overrides 'expr' from Core, but can use rules defined in Core
    rule expr -> Expr = ...
}
```

### Left Recursion
Direct left recursion is supported, which simplifies writing grammars for left-associative operators.

```nika
rule expr -> i32 =
    | l:expr "-" r:int_lit() -> { l - r }
    | v:int_lit() -> { v }
```

### Built-ins
The generator provides built-in mappings to common `syn` types:
* `ident()` -> `syn::Ident`
* `string_lit()` -> `syn::LitStr` (returns value as String)
* `int_lit()` -> `syn::LitInt` (returns value as i64)

## License
MIT or Apache License 2.0
