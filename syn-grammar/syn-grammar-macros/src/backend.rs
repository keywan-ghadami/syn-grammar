use syn_grammar_model::{Backend, BuiltIn};

pub struct SynBackend;

impl Backend for SynBackend {
    fn get_builtins() -> &'static [BuiltIn] {
        &[
            // Portable Primitives (returning portable types)
            BuiltIn {
                name: "ident",
                return_type: "syn_grammar_model::model::types::Identifier",
            },
            BuiltIn {
                name: "string",
                return_type: "syn_grammar_model::model::types::StringLiteral",
            },
            // Primitive Types (returning standard Rust types)
            BuiltIn {
                name: "char",
                return_type: "char",
            },
            BuiltIn {
                name: "bool",
                return_type: "bool",
            },
            // Integers
            BuiltIn {
                name: "i8",
                return_type: "i8",
            },
            BuiltIn {
                name: "i16",
                return_type: "i16",
            },
            BuiltIn {
                name: "i32",
                return_type: "i32",
            },
            BuiltIn {
                name: "i64",
                return_type: "i64",
            },
            BuiltIn {
                name: "i128",
                return_type: "i128",
            },
            BuiltIn {
                name: "isize",
                return_type: "isize",
            },
            BuiltIn {
                name: "u8",
                return_type: "u8",
            },
            BuiltIn {
                name: "u16",
                return_type: "u16",
            },
            BuiltIn {
                name: "u32",
                return_type: "u32",
            },
            BuiltIn {
                name: "u64",
                return_type: "u64",
            },
            BuiltIn {
                name: "u128",
                return_type: "u128",
            },
            BuiltIn {
                name: "usize",
                return_type: "usize",
            },
            // Floats
            BuiltIn {
                name: "f32",
                return_type: "f32",
            },
            BuiltIn {
                name: "f64",
                return_type: "f64",
            },
            // Alternative Bases
            BuiltIn {
                name: "hex_literal",
                return_type: "u64",
            },
            BuiltIn {
                name: "oct_literal",
                return_type: "u64",
            },
            BuiltIn {
                name: "bin_literal",
                return_type: "u64",
            },
            // Spanned Primitives (returning SpannedValue<T>)
            BuiltIn {
                name: "spanned_char",
                return_type: "syn_grammar_model::model::types::SpannedValue<char>",
            },
            BuiltIn {
                name: "spanned_bool",
                return_type: "syn_grammar_model::model::types::SpannedValue<bool>",
            },
            BuiltIn {
                name: "spanned_i8",
                return_type: "syn_grammar_model::model::types::SpannedValue<i8>",
            },
            BuiltIn {
                name: "spanned_i16",
                return_type: "syn_grammar_model::model::types::SpannedValue<i16>",
            },
            BuiltIn {
                name: "spanned_i32",
                return_type: "syn_grammar_model::model::types::SpannedValue<i32>",
            },
            BuiltIn {
                name: "spanned_i64",
                return_type: "syn_grammar_model::model::types::SpannedValue<i64>",
            },
            BuiltIn {
                name: "spanned_i128",
                return_type: "syn_grammar_model::model::types::SpannedValue<i128>",
            },
            BuiltIn {
                name: "spanned_isize",
                return_type: "syn_grammar_model::model::types::SpannedValue<isize>",
            },
            BuiltIn {
                name: "spanned_u8",
                return_type: "syn_grammar_model::model::types::SpannedValue<u8>",
            },
            BuiltIn {
                name: "spanned_u16",
                return_type: "syn_grammar_model::model::types::SpannedValue<u16>",
            },
            BuiltIn {
                name: "spanned_u32",
                return_type: "syn_grammar_model::model::types::SpannedValue<u32>",
            },
            BuiltIn {
                name: "spanned_u64",
                return_type: "syn_grammar_model::model::types::SpannedValue<u64>",
            },
            BuiltIn {
                name: "spanned_u128",
                return_type: "syn_grammar_model::model::types::SpannedValue<u128>",
            },
            BuiltIn {
                name: "spanned_usize",
                return_type: "syn_grammar_model::model::types::SpannedValue<usize>",
            },
            BuiltIn {
                name: "spanned_f32",
                return_type: "syn_grammar_model::model::types::SpannedValue<f32>",
            },
            BuiltIn {
                name: "spanned_f64",
                return_type: "syn_grammar_model::model::types::SpannedValue<f64>",
            },
            // Low-level token filters (currently return syn types or ())
            BuiltIn {
                name: "alpha",
                return_type: "syn::Ident",
            },
            BuiltIn {
                name: "digit",
                return_type: "syn::LitInt",
            },
            BuiltIn {
                name: "alphanumeric",
                return_type: "syn::Ident",
            },
            BuiltIn {
                name: "hex_digit",
                return_type: "syn::LitInt",
            },
            BuiltIn {
                name: "oct_digit",
                return_type: "syn::LitInt",
            },
            BuiltIn {
                name: "any_byte",
                return_type: "syn::LitByte",
            },
            BuiltIn {
                name: "eof",
                return_type: "()",
            },
            BuiltIn {
                name: "whitespace",
                return_type: "()",
            },
            // Syn-Specific Built-ins
            BuiltIn {
                name: "rust_type",
                return_type: "syn::Type",
            },
            BuiltIn {
                name: "rust_block",
                return_type: "syn::Block",
            },
            BuiltIn {
                name: "lit_str",
                return_type: "syn::LitStr",
            },
            BuiltIn {
                name: "lit_int",
                return_type: "syn::LitInt",
            },
            BuiltIn {
                name: "lit_char",
                return_type: "syn::LitChar",
            },
            BuiltIn {
                name: "lit_bool",
                return_type: "syn::LitBool",
            },
            BuiltIn {
                name: "lit_float",
                return_type: "syn::LitFloat",
            },
            BuiltIn {
                name: "outer_attrs",
                return_type: "Vec<syn::Attribute>",
            },
            BuiltIn {
                name: "any_ident",
                return_type: "syn::Ident",
            },
            BuiltIn {
                name: "named_field",
                return_type: "syn::Field",
            },
            BuiltIn {
                name: "unnamed_field",
                return_type: "syn::Field",
            },
            BuiltIn {
                name: "visibility",
                return_type: "syn::Visibility",
            },
            BuiltIn {
                name: "generics",
                return_type: "syn::Generics",
            },
            BuiltIn {
                name: "return_type",
                return_type: "syn::ReturnType",
            },
            BuiltIn {
                name: "statements",
                return_type: "Vec<syn::Stmt>",
            },
            BuiltIn {
                name: "pat",
                return_type: "syn::Pat",
            },
            BuiltIn {
                name: "inner_attrs",
                return_type: "Vec<syn::Attribute>",
            },
            BuiltIn {
                name: "lit_byte",
                return_type: "syn::LitByte",
            },
        ]
    }
}

/// What a `syn` type written by its path (`x:syn::Type`) is called in an error
/// message.
///
/// Without this, a failed `syn::Type` reports syn's own enumeration of all
/// sixteen tokens a type may start with, and contributes nothing at all to
/// `expected one of:` - see `grammar_kit::name_syn_failure`.
///
/// A table rather than a derivation, because the mechanical camel-case split is
/// wrong for exactly the types that occur most: `ItemUse` is not "item use" and
/// `Expr` is not "expr". The split remains the fallback for the long tail of
/// `syn` types, where a slightly clumsy word still beats no word.
pub fn syn_type_expectation(path: &syn::Path) -> String {
    let Some(last) = path.segments.last() else {
        return "syn type".to_string();
    };
    let name = last.ident.to_string();
    let known = match name.as_str() {
        "Type" | "TypePath" => "Rust type",
        "Expr" => "expression",
        "Path" => "path",
        "ItemUse" => "use statement",
        "Item" => "item",
        "Stmt" => "statement",
        "Block" => "block",
        "Pat" => "pattern",
        "Ident" => "identifier",
        "Lifetime" => "lifetime",
        "Generics" => "generic parameters",
        "GenericParam" => "generic parameter",
        "ReturnType" => "return type",
        "Signature" => "function signature",
        "Macro" => "macro invocation",
        "Attribute" => "attribute",
        "Meta" => "attribute content",
        "Visibility" => "visibility",
        "Field" => "field",
        "FnArg" => "function argument",
        "LitStr" => "string literal",
        "LitInt" => "integer literal",
        "LitFloat" => "floating point literal",
        "LitChar" => "character literal",
        "LitBool" => "`true` or `false`",
        "LitByte" => "byte literal",
        "Lit" => "literal",
        _ => "",
    };
    if !known.is_empty() {
        return known.to_string();
    }
    // Fallback: `ItemStruct` -> "item struct".
    let mut words = String::new();
    for (i, c) in name.char_indices() {
        if c.is_uppercase() && i > 0 {
            words.push(' ');
        }
        words.extend(c.to_lowercase());
    }
    words
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// Every catalog entry must have a name that occurs only once.
    /// A duplicate would be silent: `iter().any(...)` in `codegen/pattern.rs`
    /// takes the first hit, the second entry would remain ineffective.
    #[test]
    fn names_are_unique() {
        let mut seen = HashSet::new();
        for b in SynBackend::get_builtins() {
            assert!(
                seen.insert(b.name),
                "Doppelter Builtin-Name im Katalog: '{}'",
                b.name
            );
        }
    }

    /// The declared return type is read in `monomorphize.rs` via
    /// `syn::parse_str::<Type>` and used for generics inference.
    /// An unparsable entry silently falls through the cracks there
    /// (`if let Ok(ty) = ...`) and only leads to an error later in the
    /// generated code.
    #[test]
    fn return_types_are_parseable() {
        for b in SynBackend::get_builtins() {
            assert!(
                syn::parse_str::<syn::Type>(b.return_type).is_ok(),
                "return type of '{}' is not a valid type: {:?}",
                b.name,
                b.return_type
            );
        }
    }

    /// The catalog is the user interface: every entry is a
    /// promise. Pinning this number forces a new builtin to be
    /// added deliberately - together with a test and a docs entry
    /// (`syn-grammar/tests/builtin_coverage_test.rs`, `SYNTAX.md`).
    #[test]
    fn catalogue_size_is_deliberate() {
        assert_eq!(
            SynBackend::get_builtins().len(),
            63,
            "The builtin catalogue has changed. Please update test and docs accordingly."
        );
    }
}
