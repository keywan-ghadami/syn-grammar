//! Fehlermeldungs-Tests fuer den Abnahme-Benchmark.
//!
//! `GOALS.md` macht `cxx-parser` zum Massstab: er "muss super funktionieren und
//! perfekte Fehlermeldungen liefern". Geprueft wurde bisher aber nur der
//! Erfolgsfall (`src/cxx_parser.rs`, `parse_complex_cxx_bridge`) - mit nackten
//! `assert_eq!`, ohne das gemeinsame Testframework und ohne eine einzige
//! Zusicherung ueber eine Fehlermeldung.
//!
//! Diese Datei schliesst die Luecke. Sie prueft den Vertrag aus
//! `docs/adr/adr13-error-message-contract.md` an echten, kaputten
//! CXX-Bridge-Eingaben - nicht an kuenstlichen Minimalgrammatiken.

use cxx_parser::CxxParser;
use syn::parse::Parser;
use syn_grammar::testing::Testable;

/// Kurzform: Quelltext parsen und in das gemeinsame `TestResult` heben.
fn parse(src: &str) -> syn_grammar::testing::TestResult<cxx_parser::FfiMod, syn::Error> {
    CxxParser::parse_top_level_mod.parse_str(src).test().with_source(src)
}

/// ADR 13, Punkt 4: der Regelstapel wird mehrzeilig von innen nach aussen
/// ausgegeben, de-snake-cased. Das ist die Eigenschaft, die eine Meldung in
/// einer verschachtelten Grammatik ueberhaupt erst verortbar macht.
#[test]
fn regelstapel_zeigt_den_weg_von_innen_nach_aussen() {
    parse(r#"mod ffi { extern "C++" { fn f(a: ); } }"#)
        .assert_failure_contains("in cxx arg")
        .assert_failure_contains("in cxx item")
        .assert_failure_contains("in extern block")
        .assert_failure_contains("in top level mod");
}

/// ADR 13, Punkt 11: Listen benennen ihr Element und den Index. Das Label kommt
/// aus `item_label="function argument"` in der Grammatik.
#[test]
fn listenfehler_nennt_element_und_index() {
    parse(r#"mod ffi { extern "C++" { fn f(a: i32, , b: i32); } }"#)
        .assert_failure_contains("expected function argument")
        .assert_failure_contains("in function argument 2");
}

/// Der Index zaehlt mit: derselbe Fehler im ersten Argument nennt Index 1.
#[test]
fn listenindex_zaehlt_mit() {
    parse(r#"mod ffi { extern "C++" { fn f(a: ); } }"#)
        .assert_failure_contains("in function argument 1");
}

/// ADR 13, Punkt 3: am Eingabeende die Praefixform statt einer nackten
/// "expected"-Meldung.
#[test]
fn eingabeende_wird_benannt() {
    parse(r#"mod ffi { extern "C++" { fn f(a: ); } }"#)
        .assert_failure_contains("unexpected end of input");
}

/// ADR 13, Punkt 3: das tatsaechlich Vorgefundene wird benannt - hier ein
/// Schluesselwort, das als Item-Name nicht in Frage kommt.
#[test]
fn gefundenes_token_wird_benannt() {
    parse(r#"mod ffi { extern "C++" { struct S; } }"#)
        .assert_failure_contains("expected identifier")
        .assert_failure_contains("found keyword `struct`");
}

/// Scheitert die aeusserste Regel sofort, bleibt der Stapel einzeilig - es wird
/// kein Kontext erfunden, den es nicht gibt.
#[test]
fn fehler_ganz_aussen_bleibt_knapp() {
    parse(r#"extern "C++" { }"#)
        .assert_failure_contains("expected `mod`")
        .assert_failure_contains("in top level mod")
        .assert_failure_not_contains("in extern block");
}

/// Ein `syn`-Typ, der ueber die Bruecke geparst wird, liefert seine eigene
/// Meldung - und bekommt trotzdem den Regelkontext der Grammatik angehaengt.
#[test]
fn syn_typ_fehler_behaelt_grammatik_kontext() {
    parse(r#"mod ffi { extern C++ { } }"#)
        .assert_failure_contains("expected string literal")
        .assert_failure_contains("in extern block");
}

/// OFFEN - dieser Test haelt eine bekannte Schwaeche fest, statt sie zu
/// verschweigen.
///
/// Bei `fn f( 123 )` scheitert das erste Listenelement an `123`. Weil
/// `separated` mit `min=0` laeuft, ist die leere Liste gueltig, und der
/// aussagekraeftige Fehler ("expected function argument") wird nur in
/// `ParseContext::furthest` gemerkt. Direkt danach scheitert das optionale
/// `","?` an derselben Stelle und wird ebenfalls gemerkt. Beide stehen am
/// gleichen Cursor, `ParseError::merge` gibt bei Gleichstand dem spaeteren den
/// Vorzug - also gewinnt der nichtssagende Trenner-Fehler.
///
/// Zugesichert wird deshalb vorerst nur, dass die Meldung ueberhaupt an der
/// richtigen Stelle steht und den Regelkontext traegt. Sobald ein
/// Item-Fehler bei Gleichstand Vorrang vor einem blossen Token-Fehler bekommt,
/// ist hier zusaetzlich "expected function argument" zu erwarten.
#[test]
fn ungueltiges_argument_wird_noch_zu_schwach_gemeldet() {
    parse(r#"mod ffi { extern "C++" { fn f( 123 ); } }"#)
        .assert_failure_contains("in cxx item")
        .assert_failure_contains("in extern block");
}
