use crate::{
    ParseContext, ParseError, ParseResult, PRIO_AGGREGATED, PRIO_LABELED, PRIO_STRUCTURAL,
};
use syn::buffer::Cursor;
use syn::parse::Parser;

/// Erlaubt das Peeken von spezifischen syn::Tokens auf einem Cursor
pub fn peek_syn<'a, F>(cursor: Cursor<'a>, peek_fn: F) -> bool
where
    F: FnOnce(syn::parse::ParseStream) -> bool,
{
    // Nur ein kleines Fenster materialisieren statt des gesamten Reststroms.
    //
    // `peek_syn` steht unter anderem im Recover-Sync-Scan INNERHALB einer
    // Schleife (`codegen/pattern.rs`); mit voller Materialisierung war das
    // quadratisch pro Recover-Punkt.
    //
    // Vertrag: `peek_fn` darf hoechstens PEEK_FENSTER Tokens weit schauen.
    // Alle erzeugten Aufrufstellen benutzen `i.peek(..)`, also ein Token -
    // bei zusammengesetzten Operatoren wie `::` bis zu drei Punkte. Vier ist
    // damit grosszuegig. Ein Peek, der weiter schaut, saehe hier ein
    // vorzeitiges Ende und lieferte `false`.
    const PEEK_FENSTER: usize = 4;
    let mut stream = proc_macro2::TokenStream::new();
    let mut lauf = cursor;
    for _ in 0..PEEK_FENSTER {
        match lauf.token_tree() {
            Some((tt, next)) => {
                stream.extend(std::iter::once(tt));
                lauf = next;
            }
            None => break,
        }
    }
    // `Parser::parse2` verlangt, dass der Parser den GESAMTEN Stream verbraucht.
    // Ein Peek verbraucht nichts, deshalb muss der Rest hier explizit geleert
    // werden - sonst scheitert parse2 bei jedem nicht-leeren Input und die
    // Funktion liefert immer `false`.
    let parser = |input: syn::parse::ParseStream| {
        let result = peek_fn(input);
        input.parse::<proc_macro2::TokenStream>()?;
        Ok(result)
    };
    Parser::parse2(parser, stream).unwrap_or(false)
}

/// Der universelle Brücken-Kombinator.
/// Verwandelt den Cursor in einen TokenStream, lässt syn parsen und rechnet
/// anschließend exakt aus, um wie viele Schritte der Cursor vorrücken muss.
/// Marker fuer Typen, die in einer Grammatik direkt als `syn::Foo` stehen duerfen.
///
/// Fachlich identisch mit `syn::parse::Parse` - der einzige Zweck ist die
/// Fehlermeldung. Der Codegenerator (`codegen/pattern.rs`) laesst jeden Pfad
/// durch, dessen erstes Segment `syn` heisst, ohne pruefen zu koennen, ob der
/// Typ ueberhaupt parsebar ist. Ohne diesen Marker bekam der Nutzer bei
/// `syn::Field` oder `syn::Attribute` einen rohen Trait-Bound-Fehler, der auf
/// generierten Code zeigte, den er nie geschrieben hat.
#[diagnostic::on_unimplemented(
    message = "`{Self}` kann in einer Grammatik nicht direkt verwendet werden",
    note = "Ein `syn::`-Typ ist in einer Grammatik nur nutzbar, wenn er `syn::parse::Parse` implementiert.",
    note = "Typen wie `syn::Field`, `syn::Attribute` oder `syn::Pat` tun das nicht - fuer sie gibt es eingebaute Regeln (`named_field`, `outer_attrs`/`inner_attrs`, `pat`).",
    note = "Fuer alles andere: eine `extern`-Regel mit eigener Parserfunktion."
)]
pub trait SynParsable: syn::parse::Parse {}

impl<T: syn::parse::Parse> SynParsable for T {}

/// Bruecke vom `Cursor` zu einem `syn`-Typ.
///
/// Materialisiert den Reststrom, laesst `syn` daraus `T` lesen und setzt den
/// Cursor um so viele Tokens weiter, wie verbraucht wurden. Der Bound ist
/// [`SynParsable`] statt `Parse`, damit ein `syn::`-Typ ohne `Parse` eine
/// verstaendliche Meldung erzeugt statt eines rohen Trait-Bound-Fehlers.
pub fn invoke_syn_parser<'a, T: SynParsable>(cursor: Cursor<'a>) -> ParseResult<'a, T> {
    invoke_parser_fn(cursor, |input| input.parse::<T>())
}

/// Der gemeinsame Rumpf hinter [`invoke_syn_parser`] und den Builtins, die einen
/// Sonderparser brauchen (`Attribute::parse_outer`, `Pat::parse_multi_...`,
/// `Block::parse_within` - Typen ohne `impl Parse`).
///
/// Materialisiert den Reststrom bis zum Ende der umschliessenden Delimiter-Gruppe
/// und laesst `syn` darauf laufen. Das bleibt O(Rest) und ist der strukturelle
/// Preis des Cursor-first-Designs: `ParseBuffer::new` ist `pub(crate)`, es gibt
/// keinen Weg vom `Cursor` zu einem `ParseStream`. Fuer Typen mit bekannter
/// Tokenzahl gibt es [`take_fixed`], fuer Einzeltoken [`take_single`] - beide
/// ohne diesen Aufwand.
pub fn invoke_parser_fn<'a, T, F>(cursor: Cursor<'a>, parse_fn: F) -> ParseResult<'a, T>
where
    F: FnOnce(syn::parse::ParseStream) -> syn::Result<T>,
{
    // Strom und Tokenzahl in EINEM Durchlauf. Vorher waren es drei: einmal
    // `token_stream()`, einmal `clone()` und einmal `into_iter().count()`.
    let mut stream = proc_macro2::TokenStream::new();
    let mut lauf = cursor;
    let mut gesamt = 0usize;
    while let Some((tt, next)) = lauf.token_tree() {
        stream.extend(std::iter::once(tt));
        lauf = next;
        gesamt += 1;
    }

    let parser = |input: syn::parse::ParseStream| {
        let val = parse_fn(input)?;
        // Zaehlen statt materialisieren: `token_tree()` ist O(1) je Schritt und
        // legt nichts an. Vorher stand hier ein zweites `token_stream()`.
        let mut rest = 0usize;
        let mut c = input.cursor();
        while let Some((_, next)) = c.token_tree() {
            c = next;
            rest += 1;
        }
        // `Parser::parse2` verlangt, dass der GESAMTE Strom verbraucht wird.
        // Ohne dieses Leeren scheitert jeder Aufruf, auf den noch Tokens folgen.
        input.parse::<proc_macro2::TokenStream>()?;
        Ok((val, rest))
    };

    match syn::parse::Parser::parse2(parser, stream) {
        Ok((val, rest)) => {
            let mut c = cursor;
            for _ in 0..(gesamt - rest) {
                if let Some((_, next)) = c.token_tree() {
                    c = next;
                }
            }
            Ok((val, c))
        }
        // Span von syn (praezise fuer die Anzeige), Fortschritt vom
        // Eintrittscursor. Am Ende der Eingabe bzw. der Gruppe traegt syns Fehler
        // nur `Span::call_site()`; dort ist der Cursor die bessere Quelle.
        Err(e) => {
            let span = if cursor.eof() {
                cursor.span()
            } else {
                e.span()
            };
            Err(ParseError::new(span, e.to_string()).with_cursor(cursor))
        }
    }
}

/// Schliesst eine Alternativenkette ab und waehlt die Meldung, die nach aussen geht.
///
/// `best` ist der beste Einzelfehler aus den Zweigen, `expected` sammelt die Labels der
/// Zweige, die schon an ihrer Grenze gescheitert sind (also ohne ein einziges Token zu
/// verbrauchen). Umsetzung von ADR 13, Punkte 6 und 7.
pub fn finish_variants<'a>(
    best: Option<ParseError<'a>>,
    mut expected: Vec<String>,
    start: Cursor<'a>,
    fallback_msg: &str,
) -> ParseError<'a> {
    // Ein Fehler, der ueber den Startpunkt hinauskam, ist aussagekraeftiger als die
    // Aufzaehlung der Alternativen an der Startstelle - dann darf `expected one of:`
    // gar nicht erst erscheinen (ADR 13, Punkt 7).
    if let Some(b) = &best {
        let kam_weiter = b.at.map(|at| at > start).unwrap_or(false);
        if kam_weiter || b.priority >= PRIO_STRUCTURAL {
            return best.unwrap();
        }
    }

    expected.sort();
    expected.dedup();

    // Was steht an der Stelle tatsaechlich? (ADR 13, Punkt 3)
    let gefunden = match start.token_tree() {
        Some((tt, _)) => {
            let t = tt.to_string();
            if t.trim().is_empty() {
                String::new()
            } else {
                format!("; found unexpected token `{}`", t)
            }
        }
        None => String::new(),
    };

    match expected.len() {
        0 => best.unwrap_or_else(|| ParseError::at_cursor(start, fallback_msg)),
        1 => ParseError::at_cursor(start, format!("expected `{}`{}", expected[0], gefunden))
            .with_priority(PRIO_LABELED),
        _ => {
            let liste = expected
                .iter()
                .map(|e| format!("`{}`", e))
                .collect::<Vec<_>>()
                .join(", ");
            ParseError::at_cursor(start, format!("expected one of: {}{}", liste, gefunden))
                .with_priority(PRIO_AGGREGATED)
        }
    }
}

/// Optionaler Parse-Versuch. Er fängt sanfte Fehler ab und reicht strukturelle Fehler hoch.
pub fn attempt_labeled<'a, T, F>(
    cursor: Cursor<'a>,
    ctx: &mut ParseContext<'a>,
    label: Option<&str>,
    parser: F,
) -> ParseResult<'a, Option<T>>
where
    F: FnOnce(Cursor<'a>, &mut ParseContext<'a>) -> ParseResult<'a, T>,
{
    let mut fork_ctx = ctx.clone();

    match parser(cursor, &mut fork_ctx) {
        Ok((val, next_cursor)) => {
            *ctx = fork_ctx; // Zustand nach erfolgreichem Parse übernehmen
            Ok((Some(val), next_cursor))
        }
        Err(mut e) => {
            // Label applizieren, falls der Fehler exakt am Startpunkt passierte.
            // Verglichen wird ueber den Cursor, nicht ueber span.start() - letzteres
            // ist im Prozedurmakro auf stable immer (0,0) und wuerde das Label dort
            // faelschlich auf JEDEN Fehler anwenden. Siehe ADR 13, Punkt 8.
            if let Some(lbl) = label {
                if e.at == Some(cursor) {
                    e.message = format!("expected {}", lbl);
                    e.priority = std::cmp::max(e.priority, PRIO_LABELED);
                }
            }

            // Merkstelle des verworfenen Klons uebernehmen, dann den eigenen
            // Fehler merken - sonst geht er beim Zuruecksetzen verloren.
            ctx.absorb(&fork_ctx);

            // Fatale / Strukturelle Fehler werden ge-bubbled
            if e.priority >= PRIO_STRUCTURAL {
                Err(e)
            } else {
                ctx.record_failure(&e);
                // Reguläres Backtracking: Wir geben den UNVERÄNDERTEN Cursor zurück
                Ok((None, cursor))
            }
        }
    }
}

/// Beschriftet einen gescheiterten Listen-Elementversuch.
///
/// Scheiterte das Element gleich an seiner Anfangsstelle, sagt seine interne Meldung
/// nichts ueber die Liste aus - dann tritt die Erwartung des Elements an ihre Stelle,
/// bei Bedarf mit der Angabe, dass die Eingabe bzw. die Gruppe zu Ende ist
/// (ADR 13, Punkt 3). Kam es dagegen voran, ist sein eigener Fehler die
/// aussagekraeftigere Meldung und bleibt unangetastet.
///
/// In beiden Faellen bleibt der Regelstapel des Fehlers erhalten - dort steht bereits
/// der Elementindex (`in item 3`).
fn label_missing_item<'a>(
    mut e: ParseError<'a>,
    at: Cursor<'a>,
    item_name: &str,
    ctx: &ParseContext<'a>,
) -> ParseError<'a> {
    if e.at == Some(at) {
        e.message = if at.eof() {
            format!("{}, expected {}", ctx.end_of_scope_msg(), item_name)
        } else {
            format!("expected {}", item_name)
        };
        e.span = at.span();
    }
    e.priority = PRIO_STRUCTURAL;
    e
}

/// Liste aus `item_parser`, getrennt durch `sep_parser`.
///
/// `min` ist die Mindestanzahl, `trailing` erlaubt einen baumelnden Trenner.
/// `item_name` benennt die Elemente in Fehlermeldungen und landet als
/// `"<item_name> <index>"` auf dem lebenden Regelstapel - daher
/// `in function parameter 2`.
pub fn parse_separated<'a, T, P, S>(
    mut cursor: Cursor<'a>,
    ctx: &mut ParseContext<'a>,
    mut item_parser: P,
    mut sep_parser: S,
    min: usize,
    trailing: bool,
    item_name: &str,
) -> ParseResult<'a, Vec<T>>
where
    P: FnMut(Cursor<'a>, &mut ParseContext<'a>) -> ParseResult<'a, T>,
    S: FnMut(Cursor<'a>, &mut ParseContext<'a>) -> ParseResult<'a, ()>,
{
    let mut items = Vec::new();

    // Erstes Element. Der Elementname liegt waehrend des Versuchs auf dem lebenden
    // Stapel - nur so traegt ein TIEF im Element gemerkter Fehler den Listenindex.
    ctx.enter_rule(&format!("{} 1", item_name));
    let erstes = item_parser(cursor, ctx);
    ctx.exit_rule();
    match erstes {
        Ok((item, next_cursor)) => {
            items.push(item);
            cursor = next_cursor;
        }
        Err(mut e) => {
            if min > 0 {
                // Hat das Element nichts verbraucht, sagt sein interner Regelstapel
                // nichts ueber den Fehler aus - dann zaehlt nur der Listenkontext.
                if e.at == Some(cursor) {
                    e.rule_stack.clear();
                }
                // Der Fehler gehoert zum ersten Element der Liste (ADR 13, Punkt 11).
                e.push_rule(&format!("{} 1", item_name));
                return Err(label_missing_item(e, cursor, item_name, ctx));
            }
            // Leere Liste ist erlaubt - der Grund, warum kein Element kam, wird
            // aber gemerkt. Sonst bleibt spaeter nur eine generische Meldung.
            // Hier wird die Meldung NICHT ersetzt, also bleibt der interne
            // Regelstapel des Elements aussagekraeftig und wird behalten.
            e.push_rule(&format!("{} 1", item_name));
            ctx.record_failure(&e);
            return Ok((items, cursor));
        }
    }

    loop {
        let mut sep_ctx = ctx.clone();

        // Separator versuchen
        sep_ctx.enter_rule("separator");
        let sep_res = sep_parser(cursor, &mut sep_ctx);
        sep_ctx.exit_rule();
        match sep_res {
            Ok((_, after_sep_cursor)) => {
                let mut item_ctx = sep_ctx.clone();

                // Item NACH Separator versuchen
                item_ctx.enter_rule(&format!("{} {}", item_name, items.len() + 1));
                let item_res = item_parser(after_sep_cursor, &mut item_ctx);
                item_ctx.exit_rule();
                match item_res {
                    Ok((item, after_item_cursor)) => {
                        items.push(item);
                        cursor = after_item_cursor;
                        *ctx = item_ctx;
                    }
                    Err(mut e) => {
                        // Siehe oben: ohne Fortschritt traegt der interne Stapel nichts bei.
                        if e.at == Some(after_sep_cursor) {
                            e.rule_stack.clear();
                        }
                        // Index des VERSUCHTEN Elements, 1-basiert.
                        e.push_rule(&format!("{} {}", item_name, items.len() + 1));
                        if trailing {
                            // Baumelnder Trenner ist erlaubt: er GEHOERT zur Liste und
                            // wird verbraucht. Ohne das blieb er im Strom stehen und
                            // die umgebende Regel scheiterte an ihm.
                            cursor = after_sep_cursor;
                            *ctx = sep_ctx;
                            ctx.record_failure(&e);
                            break;
                        } else {
                            // Weich zuruecksetzen statt hart scheitern: der Cursor
                            // bleibt VOR dem Trenner, damit eine nachfolgende Regel
                            // (etwa ein `","?`) ihn noch verarbeiten kann. Genau darauf
                            // bauen `paren(args:liste? ","?)`-Grammatiken.
                            //
                            // Der Grund wird gemerkt - passt danach doch nichts mehr,
                            // taucht er wieder auf, statt von einer generischen Meldung
                            // ersetzt zu werden. Angereichert wird der ECHTE Fehler,
                            // damit sein Regelstapel und, wenn er tiefer lag, seine
                            // Stelle erhalten bleiben.
                            let markiert = label_missing_item(e, after_sep_cursor, item_name, ctx);
                            ctx.record_failure(&markiert);
                            ctx.absorb(&item_ctx);
                            break;
                        }
                    }
                }
            }
            Err(mut e) => {
                // Kein Trenner mehr - die Liste ist fertig. Warum es hier nicht
                // weiterging, wird trotzdem gemerkt (ADR 13, Punkt 11).
                e.rule_stack.clear();
                e.push_rule("separator");
                ctx.record_failure(&e);
                ctx.absorb(&sep_ctx);
                break;
            }
        }
    }

    if items.len() < min {
        return Err(ParseError::at_cursor(
            cursor,
            format!(
                "expected at least {} {}s, found {}",
                min,
                item_name,
                items.len()
            ),
        )
        .with_priority(PRIO_STRUCTURAL));
    }

    Ok((items, cursor))
}

/// Kombinator für Wiederholungen ohne Separator.
///
/// Gegenstück zu `parse_separated`, im selben funktionalen Stil: der Cursor wird
/// per Wert weitergereicht, Backtracking heißt schlicht, den Cursor von vor dem
/// gescheiterten Versuch weiterzubenutzen. Ein struktureller Fehler (Priorität
/// >= 50) bricht die Schleife hart ab, statt sie nur zu beenden.
pub fn parse_repeated<'a, T, P>(
    mut cursor: Cursor<'a>,
    ctx: &mut ParseContext<'a>,
    mut item_parser: P,
    min: usize,
    item_name: &str,
) -> ParseResult<'a, Vec<T>>
where
    P: FnMut(Cursor<'a>, &mut ParseContext<'a>) -> ParseResult<'a, T>,
{
    let mut items = Vec::new();

    loop {
        let mut item_ctx = ctx.clone();
        item_ctx.enter_rule(&format!("{} {}", item_name, items.len() + 1));
        let item_res = item_parser(cursor, &mut item_ctx);
        item_ctx.exit_rule();
        match item_res {
            Ok((item, next_cursor)) => {
                // Kein Fortschritt trotz Erfolg -> sonst Endlosschleife.
                if next_cursor == cursor {
                    break;
                }
                items.push(item);
                cursor = next_cursor;
                *ctx = item_ctx;
            }
            Err(e) => {
                // Strukturelle/fatale Fehler durchreichen, alles andere beendet
                // die Wiederholung regulär.
                if e.priority >= PRIO_STRUCTURAL {
                    return Err(e);
                }
                // Wiederholung endet regulaer - der Grund wird gemerkt.
                ctx.record_failure(&e);
                ctx.absorb(&item_ctx);
                break;
            }
        }
    }

    if items.len() < min {
        return Err(ParseError::at_cursor(
            cursor,
            format!(
                "expected at least {} {}s, found {}",
                min,
                item_name,
                items.len()
            ),
        )
        .with_priority(PRIO_STRUCTURAL));
    }

    Ok((items, cursor))
}

/// Ein Typ, der aus genau einem Token besteht und deshalb in O(1) direkt vom
/// `Cursor` gelesen werden kann - ohne den Umweg ueber [`invoke_syn_parser`].
///
/// Der Umweg kostet pro Aufruf eine Materialisierung des Reststroms plus einen
/// kompletten neuen `TokenBuffer` in `Parser::parse2`. Bei einem Aufruf je Token
/// wird daraus quadratischer Aufwand ueber die umschliessende Delimiter-Gruppe.
/// Fuer Einzeltoken ist diese Arbeit vollstaendig ueberfluessig.
///
/// Die Fehlermeldungen sind wortgleich mit denen von syn - mehrere Tests
/// pruefen sie per Substring.
pub trait SingleToken: Sized {
    /// Liest das Token, falls es passt. `None` heisst: passt nicht.
    fn take(cursor: Cursor<'_>) -> Option<(Self, Cursor<'_>)>;
    /// Die Meldung, wenn es nicht passt - wortgleich mit syn.
    fn erwartet() -> &'static str;
}

/// O(1)-Ersatz fuer [`invoke_syn_parser`] bei [`SingleToken`]-Typen.
pub fn take_single<'a, T: SingleToken>(cursor: Cursor<'a>) -> ParseResult<'a, T> {
    match T::take(cursor) {
        Some((wert, next)) => Ok((wert, next)),
        // Am Ende der Eingabe stellt syn seiner Meldung ein
        // "unexpected end of input, " voran. Der Bruecken-Pfad hat das
        // durchgereicht; hier wird es nachgebildet, damit sich die Meldung
        // nicht aendert (`list_dx_test::test_cxx_unexpected_eof`).
        None if cursor.eof() => Err(ParseError::at_cursor(
            cursor,
            format!("unexpected end of input, {}", T::erwartet()),
        )),
        None => Err(ParseError::at_cursor(cursor, T::erwartet())),
    }
}

impl SingleToken for proc_macro2::Ident {
    fn take(cursor: Cursor<'_>) -> Option<(Self, Cursor<'_>)> {
        // `impl Parse for Ident` lehnt Schluesselwoerter ab (`accept_as_ident`).
        // Der Unterschied zu `any_ident` haengt genau daran.
        let (id, next) = cursor.ident()?;
        if akzeptiert_als_ident(&id.to_string()) {
            Some((id, next))
        } else {
            None
        }
    }
    fn erwartet() -> &'static str {
        "expected identifier"
    }
}

/// Die Schluesselwoerter, die `syn` nicht als gewoehnlichen Bezeichner
/// durchgehen laesst (`syn::ext::IdentExt::parse_any` umgeht das).
fn akzeptiert_als_ident(s: &str) -> bool {
    !matches!(
        s,
        "abstract"
            | "as"
            | "async"
            | "await"
            | "become"
            | "box"
            | "break"
            | "const"
            | "continue"
            | "crate"
            | "do"
            | "dyn"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "final"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "macro"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "override"
            | "priv"
            | "pub"
            | "ref"
            | "return"
            | "Self"
            | "self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "try"
            | "type"
            | "typeof"
            | "unsafe"
            | "unsized"
            | "use"
            | "virtual"
            | "where"
            | "while"
            | "yield"
    )
}

impl SingleToken for syn::LitBool {
    fn take(cursor: Cursor<'_>) -> Option<(Self, Cursor<'_>)> {
        // Ein `LitBool` ist kein Literal, sondern ein Ident `true`/`false`.
        let (id, next) = cursor.ident()?;
        let s = id.to_string();
        if s == "true" || s == "false" {
            Some((
                syn::LitBool {
                    value: s == "true",
                    span: id.span(),
                },
                next,
            ))
        } else {
            None
        }
    }
    fn erwartet() -> &'static str {
        "expected boolean literal"
    }
}

/// Liest ein Literal, inklusive eines fuehrenden Minuszeichens.
///
/// `-5` ist ein `LitInt` aus ZWEI Cursor-Tokens; syn behandelt das in
/// `parse_negative_lit`. Ohne diesen Schritt verlieren `i32`, `f64` und
/// Verwandte die Faehigkeit, negative Werte zu lesen.
fn lit_mit_vorzeichen(cursor: Cursor<'_>) -> Option<(syn::Lit, Cursor<'_>)> {
    if let Some((p, nach_minus)) = cursor.punct() {
        if p.as_char() == '-' {
            let (lit, next) = nach_minus.literal()?;
            let mit_minus = format!("-{}", lit);
            // Nur Zahlen duerfen ein Vorzeichen tragen.
            return match syn::Lit::new(lit) {
                syn::Lit::Int(_) | syn::Lit::Float(_) => {
                    let mut neu: proc_macro2::Literal = mit_minus.parse().ok()?;
                    neu.set_span(p.span());
                    Some((syn::Lit::new(neu), next))
                }
                _ => None,
            };
        }
    }
    let (lit, next) = cursor.literal()?;
    Some((syn::Lit::new(lit), next))
}

macro_rules! einzeltoken_literal {
    ($typ:ty, $variante:ident, $msg:literal) => {
        impl SingleToken for $typ {
            fn take(cursor: Cursor<'_>) -> Option<(Self, Cursor<'_>)> {
                match lit_mit_vorzeichen(cursor)? {
                    (syn::Lit::$variante(l), next) => Some((l, next)),
                    _ => None,
                }
            }
            fn erwartet() -> &'static str {
                $msg
            }
        }
    };
}

einzeltoken_literal!(syn::LitStr, Str, "expected string literal");
einzeltoken_literal!(syn::LitInt, Int, "expected integer literal");
einzeltoken_literal!(syn::LitFloat, Float, "expected floating point literal");
einzeltoken_literal!(syn::LitChar, Char, "expected character literal");
einzeltoken_literal!(syn::LitByte, Byte, "expected byte literal");

/// Liest genau `anzahl` Tokens vom Cursor und laesst `syn` daraus `T` parsen.
///
/// Fuer Typen, deren Tokenzahl zur Makro-Zeit feststeht - jedes Literal-Terminal
/// einer Grammatik ist so ein Fall. Gegenueber [`invoke_syn_parser`] wird nicht
/// der gesamte Reststrom materialisiert, sondern nur diese `anzahl` Tokens; der
/// `TokenBuffer`, den `Parser::parse2` daraus baut, ist entsprechend winzig.
///
/// Der Umweg ueber `syn` bleibt bewusst erhalten: die Token-Typen sind versiegelt
/// und ihre Felder unterschiedlich geformt, ein Nachbau waere fehleranfaellig.
/// So bleiben Meldungstexte und die `Spacing::Joint`-Pruefung zusammengesetzter
/// Operatoren exakt syns eigene - der `Punct` traegt sein `Spacing` mit.
pub fn take_fixed<'a, T: SynParsable>(cursor: Cursor<'a>, anzahl: usize) -> ParseResult<'a, T> {
    let mut stueck = proc_macro2::TokenStream::new();
    let mut lauf = cursor;
    let mut gelesen = 0usize;
    while gelesen < anzahl {
        match lauf.token_tree() {
            Some((tt, next)) => {
                stueck.extend(std::iter::once(tt));
                lauf = next;
                gelesen += 1;
            }
            None => break,
        }
    }

    match syn::parse::Parser::parse2(T::parse, stueck) {
        Ok(wert) => Ok((wert, lauf)),
        Err(e) => {
            let span = if cursor.eof() {
                cursor.span()
            } else {
                e.span()
            };
            Err(ParseError::new(span, e.to_string()).with_cursor(cursor))
        }
    }
}
