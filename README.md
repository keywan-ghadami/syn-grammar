# syn-grammar

**A parser generator for Rust that compiles EBNF-like grammars into `syn::parse::Parse` implementations.**

`syn-grammar` bridges the gap between grammar-based parser generators (like `pest` or `lalrpop`) and hand-written recursive descent parsers using `syn`. It allows you to define your language syntax declaratively in `.grammar` files while embedding raw Rust code for the AST construction.

> **Note:** This project was originally built as the Stage 0 Bootstrap tool for the **Nikaia** programming language but has been extracted into a standalone library.

## Features

* **Syn-Powered:** Generates robust Rust code using `syn` and `quote`. The output depends only on `syn`, not on this library.
* **Grammar Inheritance:** Supports modular language definition. You can define a `Core` grammar and extend it in `Advanced` grammars (`grammar Child : Parent`).
* **Rust Actions:** Embed arbitrary Rust code in your grammar rules (`-> { ... }`) to construct your AST nodes directly.
* **Build Integration:** Designed to run in `build.rs` to generate parsers at compile time.

## Usage

### 1. Define your Grammar
Create a file named `src/grammar/Calc.grammar`.

```nika
grammar Calc {
    // Top-level rule returning a Rust type (e.g., i32)
    pub rule expr -> i32 = 
        | "add" a:int_lit() b:int_lit() -> { a + b }
        | "sub" a:int_lit() b:int_lit() -> { a - b }
        | v:int_lit() -> { v }
        
    // 'int_lit' is a built-in that maps to syn::LitInt
}
```

### 2. Add to `build-dependencies`
In your `Cargo.toml`:

```toml
[build-dependencies]
syn-grammar = "0.1"
```

### 3. Configure `build.rs`
Use the generator to compile the grammar into a Rust file during the build.

```rust
// build.rs
use syn_grammar::Generator;
use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("calc_parser.rs");

    // Initialize generator pointing to the folder containing .grammar files
    let gen = Generator::new("src/grammar");
    
    // Resolve inheritance and generate code
    let code = gen.generate("Calc.grammar").expect("Failed to generate parser");

    fs::write(&dest_path, code.to_string()).unwrap();
    println!("cargo:rerun-if-changed=src/grammar");
}
```

### 4. Include in your Code
Import the generated parser code in your library.

```rust
// src/lib.rs
use syn::{parse_macro_input, parse::ParseStream};

// Include the generated code (defines functions like `parse_expr`)
include!(concat!(env!("OUT_DIR"), "/calc_parser.rs"));

pub fn parse_calculation(input: ParseStream) -> syn::Result<i32> {
    // Call the entry point rule defined in the grammar
    parse_expr(input)
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
// In Extended.grammar
grammar Extended : Core {
    // Overrides 'expr' from Core, but can use rules defined in Core
    rule expr -> Expr = ...
}
```

### Built-ins
The generator provides built-in mappings to common `syn` types:
* `ident()` -> `syn::Ident`
* `string_lit()` -> `syn::LitStr` (returns value as String)
* `int_lit()` -> `syn::LitInt` (returns value as i64)

## License
MIT or Apache License 2.0

