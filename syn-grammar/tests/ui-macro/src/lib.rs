//! Ein echtes Prozedurmakro auf Basis einer `syn-grammar`-Grammatik.
//!
//! Zweck: der einzige Pfad, auf dem sich das Verhalten im *realen* Makro pruefen
//! laesst. Die gesamte uebrige Testsuite laeuft ueber `Parser::parse_str` und
//! damit ueber den proc-macro2-**Fallback**, der echte Zeilen und Spalten hat.
//! In einem echten Prozedurmakro auf stable liefert `Span::start()` dagegen fuer
//! jeden Span `(0,0)` - alles, was daran haengt, ist ueber `parse_str`
//! unsichtbar. Siehe `GOALS.md` und ADR 13, Punkt 14.

use proc_macro::TokenStream;

// Ein proc-macro-Crate darf ausser den Makros selbst nichts exportieren.
// `grammar!` erzeugt ein `pub mod Demo` - eingewickelt in ein privates Modul ist
// es aus dem Crate-Wurzelmodul heraus nicht sichtbar und damit zulaessig.
mod grammatik {
    syn_grammar::grammar! {
    grammar Demo {
        /// `let <name> = <zahl>;` - klein genug, um die Meldung lesbar zu halten,
        /// und mit genug Struktur fuer einen mehrstufigen Regelstapel.
        pub rule zuweisung -> i32 = "let" name:ident "=" v:wert ";" -> {
            let _ = name;
            v
        }

        rule wert -> i32 = i:i32 -> { i }

        /// Nur fuer den Adjazenz-Fall: `::` muss ein zusammenhaengender Operator
        /// sein. `a : : b` darf NICHT passen.
        pub rule pfad -> String = a:ident "::" b:ident -> {
            format!("{}::{}", a, b)
        }
    }
    }
}

use grammatik::Demo;

/// Parst `let x = 1;` und gibt die Zahl als Ausdruck zurueck.
#[proc_macro]
pub fn zuweisung(input: TokenStream) -> TokenStream {
    ausfuehren(input, Demo::parse_zuweisung)
}

/// Parst `a::b` und gibt den Pfad als String-Literal zurueck.
#[proc_macro]
pub fn pfad(input: TokenStream) -> TokenStream {
    ausfuehren(input, |s| Demo::parse_pfad(s).map(|p| p.len() as i32))
}

/// Gemeinsamer Rumpf: Erfolg wird zu einem harmlosen Ausdruck, ein Fehler zu
/// `compile_error!`. Genau so, wie ein echtes Makro es taete - und genau so
/// landet die Meldung in der `.stderr`-Datei von trybuild.
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
