# Grammar Syntax Reference

This document serves as the reference for the **Grammar Definition Language** shared by all backends (`syn-grammar`, `winnow-grammar`).

## Defining Grammars

Grammars are defined using the `grammar!` macro. A grammar block contains a set of rules.

```rust
# use syn_grammar::grammar;
# fn main() {
grammar! {
    grammar MyGrammar {
        start = "hello"
    }
}
# }
```

## Rules

A rule consists of a name, a return type, a pattern, and an action block.

```text
    name -> ReturnType = pattern -> { action_code }
```

- **`name`**: The name of the rule.
- **`ReturnType`**: The Rust type returned by the rule.
- **`pattern`**: The grammar pattern to match.
- **`action_code`**: A Rust block that constructs the return value.

### Lexical vs. Syntactic Rules (Case Sensitivity)

The casing of a rule's name determines its whitespace handling:

- **Syntactic Rules (lowercase)**: Rule names starting with a **lowercase** letter (e.g., `rule expression`) allow implicit whitespace between patterns.
- **Lexical Rules (UPPERCASE)**: Rule names starting with an **uppercase** letter (e.g., `rule IDENTIFIER`) are **lexical**. They do **not** allow implicit whitespace between patterns.

```rust
# use syn_grammar::grammar;
# fn main() {
#     grammar! {
#         grammar Test {
// Syntactic: matches "a + b"
 add = "a" "+" "b"

// Lexical: matches "ab", but NOT "a b"
 AB = "a" "b" 
#         }
#     }
# }
```

## Syntax Guide

### Sequences & Bindings
Match a sequence of patterns. Use `name:pattern` to bind the result to a variable available in the action block.

```rust
# use syn_grammar::grammar;
# use syn_grammar::types::Identifier;
# fn main() {
#     grammar! {
#         grammar Test {
 assignment -> (Identifier, i32) = 
    name:ident "=" val:i32 -> { (name, val) }
#         }
#     }
# }
```

### Alternatives
Match one of several alternatives using `|`. The first one that matches wins.

```rust
# use syn_grammar::grammar;
# fn main() {
#     grammar! {
#         grammar Test {
 choice -> bool = 
    "yes" -> { true }
  | "no"  -> { false }
#         }
#     }
# }
```

### Repetitions
- `pattern*`: Match zero or more times. Returns a `Vec`.
- `pattern+`: Match one or more times. Returns a `Vec`.
- `pattern?`: Match zero or one time. Returns an `Option`.

```rust
# use syn_grammar::grammar;
# fn main() {
#     grammar! {
#         grammar Test {
 list -> Vec<i32> = elements:i32* -> { elements }
#         }
#     }
# }
```

### Delimiters
To match literal delimiters (parentheses, brackets, braces) in the input, use the specific delimiter syntax. This avoids ambiguity with grouping parentheses.

- `paren(pattern)`: Matches `( pattern )`.
- `[ pattern ]`: Matches `[ pattern ]`.
- `{ pattern }`: Matches `{ pattern }`.

```rust
# use syn_grammar::grammar;
# fn main() {
#     grammar! {
#         grammar Test {
 tuple -> (i32, i32) = 
    paren(a:i32 "," b:i32) -> { (a, b) }
#         }

#     }
# }
```

Use standard parentheses `(...)` **only** for logical grouping of patterns (e.g., inside an alternative).

```rust
# use syn_grammar::grammar;
# fn main() {
#     grammar! {
#         grammar Test {
 group = ("a" | "b") "c"
#         }
#     }
# }
```

### Literals
Match specific tokens or text using string literals.

```rust
# use syn_grammar::grammar;
# fn main() {
#     grammar! {
#         grammar Test {
 kw  = "fn" "name"
#         }
#     }
# }
```

For matching Rust literals as values, use the `lit_*` built-ins:
- `lit_str`: Matches a string literal.
- `lit_int`: Matches an integer literal.
- `lit_char`: Matches a character literal.
- `lit_bool`: Matches `true` or `false`.
- `lit_float`: Matches a floating-point literal.

### Built-in Primitives
The following primitives are "portable" and expected to be available in all backends, though their exact return types may vary slightly (e.g., `String` vs `syn::Ident`).

| Parser | Description |
|---|---|
| `ident` | An identifier (e.g., variable name). |
| `string` | A string literal (same as `lit_str`). |
| `u32` | Unsigned 32-bit integer. |
| `i32` | Signed 32-bit integer. |
| `bool` | Boolean (`true` or `false`). |
| `alpha` | Alphabetic characters. |
| `digit` | Numeric digits. |
| `whitespace` | Explicit whitespace matching. |
| `eof` | End of input. |

*Note: Backends may provide additional specialized built-ins.*

### Spanned Primitives

Every numeric and character primitive has a `spanned_` variant returning
`syn_grammar::types::SpannedValue<T>`, which carries both the parsed `value` and
its `span`:

```rust
# use syn_grammar::grammar;
# use syn_grammar::types::SpannedValue;
# fn main() {
#     grammar! {
#         grammar Test {
 versioned -> SpannedValue<u32> = v:spanned_u32 -> { v }
#         }
#     }
# }
```

Available: `spanned_char`, `spanned_bool`, `spanned_f32`, `spanned_f64`, and
`spanned_` variants of every integer width (`spanned_i8` … `spanned_i128`,
`spanned_isize`, `spanned_u8` … `spanned_u128`, `spanned_usize`).

Use these when you need the location of a value for a later diagnostic of your
own. For a span *without* wrapping the value, see the `@` operator below.

### Span Binding (`@`)

`name:rule @ span_var` binds the parsed value **and** its span at once. The rule
must return a type implementing `syn::spanned::Spanned`:

```rust
# use syn_grammar::grammar;
# fn main() {
#     grammar! {
#         grammar Test {
 decl -> String = "let" name:any_ident @ pos -> {
     let _ = pos;
     name.to_string()
 }
#         }
#     }
# }
```

This is the usual way to produce your own `syn::Error` from an action block that
points at the right token.

## List Parsing

The grammar provides convenient built-in functions for parsing lists and repetitions.

- **`separated(item, separator)`**: Parses a list of `item`s separated by a `separator`.
- **`repeated(item)`**: Parses a sequence of `item`s (whitespace-separated in syntactic rules).

Both functions return a `Vec` of the `item`'s result type. They can be customized with the following named arguments:

- `min = <usize>`: The minimum number of items required.
- `trailing = <bool>`: Whether a trailing separator is allowed (defaults to `false`).
- `error = <&str>`: A custom error message to display on failure.

```rust
# use syn_grammar::grammar;
# use syn_grammar::types::StringLiteral;
# fn main() {
#     grammar! {
#         grammar Test {
// Matches "a", "b", "c"
comma_list -> Vec<StringLiteral> = items:separated(string, ",") -> { items }

// Matches "a", "b", "c",
trailing_comma_list -> Vec<StringLiteral> = items:separated(string, ",", trailing = true) -> { items }

// Matches at least two items
min_two_items -> Vec<StringLiteral> = items:separated(string, ",", min = 2) -> { items }

// Matches "a" "b" "c"
space_list -> Vec<StringLiteral> = items:repeated(string) -> { items }
#         }
#     }
# }
```

## Operators

### Cut Operator (`=>`)
The cut operator commits to the current alternative. If the pattern *before* the `=>` matches, the parser will **not** backtrack to other alternatives if the pattern *after* the `=>` fails.

```rust
# use syn_grammar::grammar;
# use syn_grammar::types::Identifier;
# #[derive(Debug)]
# pub enum Stmt {
#     Let(Identifier, Box<Expr>),
#     Expr(Box<Expr>),
# }
# #[derive(Debug)]
# pub struct Expr;
# fn main() {
#     grammar! {
#         grammar Test {
 stmt -> Stmt =
    "let" => name:ident "=" e:expr -> { Stmt::Let(name, Box::new(e)) }
  | e:expr -> { Stmt::Expr(Box::new(e)) }

expr -> Expr = i32 -> { Expr }
#         }
#     }
# }
```

### Lookahead (`peek`, `not`)
- `peek(pattern)`: Succeeds if `pattern` matches, but does not consume input.
- `not(pattern)`: Succeeds if `pattern` does *not* match.

```rust
# use syn_grammar::grammar;
# fn main() {
#     grammar! {
#         grammar Test {
check = "a" peek("b")
#         }
#     }
# }
```

### Lexical Control (`lex`, `spaced`)
- `lex(pattern)`: Forces a **lexical context** (no implicit whitespace) for the duration of the pattern.
- `spaced(pattern)`: Forces a **syntactic context** (implicit whitespace allowed) even inside a lexical rule.

```rust
# use syn_grammar::grammar;
# fn main() {
#     grammar! {
#         grammar Test {
 word  = lex(alpha+)
 CAST_OPERATOR = "as" spaced("<" T ">") 
rule T  = "bool"
#         }
#     }
# }
```

### Special (`until`, `count`, `eof`, `fail`, `recover`)

- **`until(terminator)`**: Consumes tokens until `terminator` is matched. The terminator is not consumed.
- **`count(pattern)`**: Returns the number of times `pattern` matched (as `usize`).
- **`eof`**: Succeeds only at the end of the input.
- **`fail("message")`**: Explicitly fails with a custom error message.
- **`recover(rule, sync)`**: If `rule` fails, skips input until `sync` token is found.

`until` captures everything up to a terminator — useful for unstructured content:

```rust
# use syn_grammar::grammar;
# fn main() {
#     grammar! {
#         grammar Test {
 directive -> proc_macro2::TokenStream = "#" body:until(";") ";" -> { body }
#         }
#     }
# }
```

`count` yields how often a pattern matched, without collecting the values:

```rust
# use syn_grammar::grammar;
# fn main() {
#     grammar! {
#         grammar Test {
 stars -> usize = n:count("*") -> { n }
#         }
#     }
# }
```

`fail` reports its message verbatim, without an `expected` prefix. Use it for
checks the grammar itself cannot express:

```rust
# use syn_grammar::grammar;
# fn main() {
#     grammar! {
#         grammar Test {
 visibility = "pub" | "priv" fail("`priv` was removed in Rust 2018")
#         }
#     }
# }
```

`recover` keeps parsing after an error instead of aborting at the first one —
the parser skips to the synchronisation token and continues:

```rust
# use syn_grammar::grammar;
# fn main() {
#     grammar! {
#         grammar Test {
 program -> Vec<Option<String>> = items:recover(statement, ";")* -> { items }
 statement -> String = "let" name:ident -> { name.to_string() }
#         }
#     }
# }
```

## Error Messages

Error message quality is the point of this library, and two operators control it
directly. See [`docs/ERROR_HANDLING.md`](../docs/ERROR_HANDLING.md) for how the
engine picks a message, and
[`docs/adr/adr13-error-message-contract.md`](../docs/adr/adr13-error-message-contract.md)
for the binding contract.

### Alternative Labels (`#`)

By default a failing alternative is described by its first token. `# "..."`
replaces that with a human-readable name. The label is placed **after the
pattern and before the action block**.

```rust
# use syn_grammar::grammar;
# fn main() {
#     grammar! {
#         grammar Test {
 value -> String = i:i32 # "a number" -> { i.to_string() }
                | s:string # "a string" -> { s.value }
#         }
#     }
# }
```

Without labels a failure reports ``expected one of: `a`, `b` ``; with them it
reports `expected one of: a number, a string`. Labels also work inside groups:
`("a" # "A" | "b" # "B")`.

An alternative that fails **after consuming input** keeps its own detailed
message — the label only stands in when the alternative failed right at its
start. That is what keeps a deep, specific error from being replaced by a
shallow summary.

### List Item Labels (`item_label`)

`separated` and `repeated` name their elements and count them:

```rust
# use syn_grammar::grammar;
# fn main() {
#     grammar! {
#         grammar Test {
 params -> Vec<String> =
     items:separated(param, ",", item_label="function parameter") -> { items }
 param -> String = i:ident -> { i.to_string() }
#         }
#     }
# }
```

A failure in the second element then reads
`expected function parameter … in function parameter 2` instead of the
anonymous `expected item … in item 2`.

### Rule Context

Every failure carries the chain of rules it happened in, innermost first, with
underscores turned into spaces:

```text
expected integer literal at column 4 (line 1)
in term
in expression
```

The position is only printed when the span actually has line/column data. Inside
a real proc macro on stable Rust it does not (rustc underlines the span in the
editor instead) — see [`GOALS.md`](../GOALS.md).

## Advanced Features

### Rule Arguments
Rules can accept arguments to pass context or configuration.

```rust
# use syn_grammar::grammar;
# fn main() {
#     grammar! {
#         grammar Test {
main -> i32 = "start" v:value(offset=10) -> { v }
value(offset: i32) -> i32 = i:i32 -> { i + offset }
#         }
#     }
# }
```

### Generic Rules
Define reusable rules with generic types and parser parameters.

```rust
# use syn_grammar::grammar;
# fn main() {
#     grammar! {
#         grammar Test {
list<T>(item) -> Vec<T> = items:item* -> { items }
integers -> Vec<i32> = l:list(item=i32) -> { l }
#         }
#     }
# }
```

### Left Recursion
Direct left recursion is automatically detected and compiled into an iterative loop, making expression parsing natural.

```rust
# use syn_grammar::grammar;
# fn main() {
#     grammar! {
#         grammar Test {
 expr -> i32 = 
    l:expr "+" r:term -> { l + r }
  | t:term            -> { t }

term -> i32 = i:i32 -> {i}
#         }
#     }
# }
```

### Shadowing Detection
The compiler checks for unreachable alternatives (e.g., if a prefix shadows a longer rule) and emits warnings or errors.
