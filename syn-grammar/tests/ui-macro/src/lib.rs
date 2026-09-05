//! A real procedural macro based on a `syn-grammar` grammar.
//!
//! Purpose: the only path on which the behaviour in the *real* macro can be
//! checked. The entire rest of the test suite runs through `Parser::parse_str`
//! and thus through the proc-macro2 **fallback**. Everything that differs
//! between fallback and real macro is invisible there - first and foremost the
//! question whether spans carry positions (only from Rust 1.88 on, see `GOALS.md`).
//! ADR 13, point 14.

use proc_macro::TokenStream;

// A proc-macro crate must not export anything except the macros themselves.
// `grammar!` generates a `pub mod Demo` - wrapped in a private module it is
// not visible from the crate root and therefore permitted.
mod grammatik {
    syn_grammar::grammar! {
    grammar Demo {
        /// `let <name> = <number>;` - small enough to keep the message readable,
        /// and with enough structure for a multi-level rule stack.
        pub rule zuweisung -> i32 = "let" name:ident "=" v:wert ";" -> {
            let _ = name;
            v
        }

        rule wert -> i32 = i:i32 -> { i }

        /// Only for the adjacency case: `::` must be a joint operator.
        /// `a : : b` must NOT match.
        pub rule pfad -> String = a:ident "::" b:ident -> {
            format!("{}::{}", a, b)
        }
    }
    }
}

use grammatik::Demo;

/// Parses `let x = 1;` and returns the number as an expression.
#[proc_macro]
pub fn zuweisung(input: TokenStream) -> TokenStream {
    ausfuehren(input, Demo::parse_zuweisung)
}

/// Parses `a::b` and returns the path as a string literal.
#[proc_macro]
pub fn pfad(input: TokenStream) -> TokenStream {
    ausfuehren(input, |s| Demo::parse_pfad(s).map(|p| p.len() as i32))
}

/// Shared body: success becomes a harmless expression, an error becomes
/// `compile_error!`. Exactly as a real macro would do it - and exactly so the
/// message ends up in trybuild's `.stderr` file.
fn ausfuehren<F>(input: TokenStream, parser: F) -> TokenStream
where
    F: FnOnce(syn::parse::ParseStream) -> syn::Result<i32>,
{
    match syn::parse::Parser::parse2(parser, input.into()) {
        Ok(v) => quote_i32(v),
        Err(e) => e.to_compile_error().into(),
    }
}

fn quote_i32(v: i32) -> TokenStream {
    let lit = proc_macro2::Literal::i32_unsuffixed(v);
    proc_macro2::TokenStream::from(proc_macro2::TokenTree::Literal(lit)).into()
}
