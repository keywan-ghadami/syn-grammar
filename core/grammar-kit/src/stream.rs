//! Der stromgefuehrte Teil der Laufzeit (ADR 15, Stufe 3).
//!
//! Bis Stufe 2 arbeitete der erzeugte Parser ausschliesslich auf
//! [`syn::buffer::Cursor`]. Fuer echte syn-AST-Typen gab es damit keinen Weg zu
//! einem `ParseStream`, also musste pro Aufruf der Reststrom materialisiert und
//! daraus ein kompletter neuer `TokenBuffer` gebaut werden - quadratisch in der
//! Laenge der umschliessenden Delimiter-Gruppe.
//!
//! Hier laeuft der Rumpf stattdessen auf einem `ParseBuffer`, der genau einmal
//! gebaut wird. Ein `syn::Type` kostet damit `input.parse::<T>()`, also
//! O(Laenge des Typs) statt O(Rest).
//!
//! Die Cursor-Primitiven bleiben unveraendert: [`schritt`] laesst sie in einer
//! `step`-Episode laufen und rueckt den Strom um genau das Ergebnis vor.

use crate::{ParseError, ParseResult, PRIO_STRUCTURAL};
use proc_macro2::{Delimiter, Span};
use syn::buffer::Cursor;
use syn::parse::discouraged::Speculative;
use syn::parse::ParseBuffer;

/// Der Eingabestrom einer Regel.
///
/// Bewusst `ParseBuffer<'a>` und **nicht** syns Alias
/// `ParseStream<'a> = &'a ParseBuffer<'a>`. Der Alias setzt die Lebensdauer der
/// Referenz mit der der Tokens gleich; eine Regel wuerde dann
/// `fn regel<'a>(input: &'a ParseBuffer<'a>)` heissen, und ein `input.fork()`
/// lebt nur bis zum Ende des Stapelrahmens. Damit waere `'a` auf diesen Rahmen
/// verkuerzt und ein `ParseError<'a>` aus einem Fork koennte den Aufruf nicht
/// mehr verlassen - genau die Fehler, die die Auswahl braucht.
///
/// Mit `&Strom<'a>` bleibt die Referenz-Lebensdauer frei und `'a` haengt allein
/// an den Tokens. Das ist die Voraussetzung fuer die gesamte Fehlerauswahl.
pub type Strom<'a> = ParseBuffer<'a>;

/// Ergebnis eines Parseschritts auf dem Strom.
///
/// Gegenstueck zu [`ParseResult`]: dort ist der Fortschritt der zurueckgegebene
/// Cursor, hier steckt er im Strom selbst.
pub type StreamResult<'a, T> = Result<T, ParseError<'a>>;

/// Laesst eine Cursor-Primitive auf dem Strom laufen und rueckt ihn vor.
///
/// `ParseBuffer::step` ist der einzige oeffentliche Weg, die Position eines
/// Stroms zu setzen, und er verlangt, dass die Closure fuer **jede**
/// Lebensdauer `'c` funktioniert. Ein `ParseError<'c>` kann deshalb nicht
/// herausgereicht werden - er traegt einen `Cursor<'c>`.
///
/// Also wird der Fehler ohne seinen Cursor durch die Schranke getragen und
/// draussen neu an die Eintrittsstelle gehaengt. Das ist keine Naeherung: die
/// Primitiven ([`crate::take_single`], die Zeichenfilter) melden ihren Fehler
/// ohnehin an der Eintrittsstelle.
pub fn schritt<'a, T, F>(input: &Strom<'a>, f: F) -> StreamResult<'a, T>
where
    F: for<'c> FnOnce(Cursor<'c>) -> ParseResult<'c, T>,
{
    let hier = input.cursor();
    let mut merk: Option<(Span, String, u8, bool)> = None;
    let ergebnis = input.step(|sc| match f(*sc) {
        Ok((wert, danach)) => Ok((wert, danach)),
        Err(e) => {
            let span = e.span;
            merk = Some((span, e.message, e.priority, e.is_fatal));
            // Der Text wird draussen aus `merk` genommen; dieser Fehler dient
            // nur dazu, `step` das Vorruecken zu verweigern.
            Err(syn::Error::new(span, "unreachable"))
        }
    });

    match ergebnis {
        Ok(wert) => Ok(wert),
        Err(syn_fehler) => Err(match merk {
            Some((span, message, priority, fatal)) => {
                let mut e = ParseError::new(span, message)
                    .with_cursor(hier)
                    .with_priority(priority);
                e.is_fatal = fatal;
                e
            }
            // Kann nur eintreten, wenn `step` selbst scheitert, nicht die
            // Closure. Defensiv, nicht der Normalfall.
            None => ParseError::new(syn_fehler.span(), syn_fehler.to_string()).with_cursor(hier),
        }),
    }
}

/// Liest einen syn-AST-Typ vom Strom.
///
/// **Der Kern von Stufe 3.** Vorher lief das ueber `invoke_syn_parser`, das den
/// Reststrom materialisierte und `Parser::parse2` daraus einen neuen
/// `TokenBuffer` bauen liess - O(Rest) je Aufruf. Jetzt ist es ein gewoehnlicher
/// `parse`-Aufruf auf dem bestehenden Puffer, also O(Laenge des Typs).
///
/// Der Bound ist [`crate::SynParsable`] statt `Parse`, damit ein `syn::`-Typ
/// ohne `Parse` eine verstaendliche Meldung erzeugt statt eines rohen
/// Trait-Bound-Fehlers auf generiertem Code.
///
/// **Nach einem Fehler ist der Strom moeglicherweise vorgerueckt.** Anders als
/// beim Cursor ist Zuruecksetzen hier nicht gratis; der Aufrufer muss auf einer
/// Gabel arbeiten, wenn er zuruecksetzen koennen will. Der Codegenerator tut
/// das an jeder Ruecksetzstelle.
pub fn parse_syn<'a, T: crate::SynParsable>(input: &Strom<'a>) -> StreamResult<'a, T> {
    let hier = input.cursor();
    input.parse::<T>().map_err(|e| fehler_von_syn(e, hier))
}

/// Wie [`parse_syn`], aber mit einem Sonderparser - fuer Typen ohne `impl Parse`
/// (`syn::Attribute`, `syn::Pat`) oder mit abweichendem Einstieg
/// (`Block::parse_within`, `Ident::parse_any`).
pub fn parse_mit<'a, T, F>(input: &Strom<'a>, parser: F) -> StreamResult<'a, T>
where
    F: FnOnce(syn::parse::ParseStream) -> syn::Result<T>,
{
    let hier = input.cursor();
    parser(input).map_err(|e| fehler_von_syn(e, hier))
}

/// Uebernimmt einen `syn::Error`.
///
/// Span von syn - der ist fuer die Anzeige praeziser. Fortschritt von der
/// Eintrittsstelle. Am Ende der Eingabe bzw. der Gruppe traegt syns Fehler nur
/// `Span::call_site()`; dort ist der Cursor die bessere Quelle.
fn fehler_von_syn<'a>(e: syn::Error, hier: Cursor<'a>) -> ParseError<'a> {
    let span = if hier.eof() { hier.span() } else { e.span() };
    ParseError::new(span, e.to_string()).with_cursor(hier)
}

/// Eine Gabel des Stroms - der Ruecksetzpunkt.
///
/// Solange die Gabel nicht per [`uebernehmen`] eingespielt wird, bleibt der
/// Strom stehen. Das ersetzt den Cursor-Copy des alten Designs; es kostet eine
/// kleine `Rc`-Allokation, dafuer entfaellt der `TokenBuffer`-Bau.
pub fn gabel<'a>(input: &Strom<'a>) -> Strom<'a> {
    input.fork()
}

/// Spielt eine erfolgreiche Gabel in den Strom ein. Laut syn O(1), unabhaengig
/// vom Abstand.
pub fn uebernehmen<'a>(input: &Strom<'a>, gabel: &Strom<'a>) {
    input.advance_to(gabel);
}

/// Steigt in eine Delimiter-Gruppe ab.
///
/// Liefert die Spanne der Klammern und den Strom ihres Inhalts; der aeussere
/// Strom steht danach hinter der Gruppe.
///
/// Benutzt `syn::__private::parse_{parens,braces,brackets}` - die Funktionen
/// hinter den Makros `parenthesized!`/`braced!`/`bracketed!`. Sie sind
/// `#[doc(hidden)]`, also ohne Semver-Versprechen, und deshalb hier an genau
/// einer Stelle gekapselt (wie `Token::peek` in [`crate::peek_syn`]).
///
/// Die Makros selbst gehen nicht: ihr Fehlerpfad ist ein nacktes
/// `return Err(syn::Error)`, was eine Funktion voraussetzt, die genau diesen
/// Fehlertyp zurueckgibt. `AnyDelimiter::parse_any_delimiter` geht ebenfalls
/// nicht - seine Rueckgabe ist auf die Lebensdauer von `&self` verkuerzt, womit
/// kein Fehler aus der Gruppe mehr nach aussen tragen wuerde.
pub fn gruppe<'a>(
    input: &Strom<'a>,
    delimiter: Delimiter,
) -> StreamResult<'a, (proc_macro2::extra::DelimSpan, Strom<'a>)> {
    let hier = input.cursor();
    let ergebnis = match delimiter {
        Delimiter::Parenthesis => {
            syn::__private::parse_parens(input).map(|g| (g.token.span, g.content))
        }
        Delimiter::Brace => syn::__private::parse_braces(input).map(|g| (g.token.span, g.content)),
        Delimiter::Bracket => {
            syn::__private::parse_brackets(input).map(|g| (g.token.span, g.content))
        }
        Delimiter::None => Err(syn::Error::new(hier.span(), "expected delimited group")),
    };
    ergebnis.map_err(|_| {
        ParseError::at_cursor(hier, "expected delimited group").with_priority(PRIO_STRUCTURAL)
    })
}

/// Nimmt genau einen Token-Tree vom Strom.
///
/// Fuer die Stellen, die roh ueber Tokens laufen: `until(..)` sammelt sie,
/// `recover(..)` ueberspringt sie bis zur Synchronisationsmarke.
pub fn token_nehmen<'a>(input: &Strom<'a>) -> StreamResult<'a, proc_macro2::TokenTree> {
    schritt(input, |cursor| match cursor.token_tree() {
        Some((tt, danach)) => Ok((tt, danach)),
        None => Err(ParseError::at_cursor(cursor, "unexpected end of input")),
    })
}
