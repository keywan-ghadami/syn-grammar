//! Generated code must not trip the lints a user crate switches on.
//!
//! `#![warn(missing_docs)]` is standard in a published library - this workspace
//! uses it in every crate of its own. The `grammar!` module is code the user
//! never wrote, so a warning in it is one they cannot fix; with `-D warnings`
//! in their CI it is a build failure. The generated module carries a doc
//! comment and allows the lint for everything inside it that cannot be
//! documented from outside (`kw`, the structs from `syn::custom_keyword!`).
#![deny(missing_docs)]

use syn::parse::Parser;
use syn_grammar::grammar;

grammar! {
    grammar Documented {
        pub rule greeting -> String = "hello" who:addressee -> { who }
        // A private rule too: it generates a function of its own.
        rule addressee -> String = i:ident -> { i.to_string() }
    }
}

/// The grammar above must compile under `deny(missing_docs)`; this only proves
/// it also still parses.
#[test]
fn a_grammar_compiles_under_deny_missing_docs() {
    assert_eq!(
        Documented::parse_greeting.parse_str("hello world").unwrap(),
        "world"
    );
    assert!(!Documented::GRAMMAR_NAME.is_empty());
}
