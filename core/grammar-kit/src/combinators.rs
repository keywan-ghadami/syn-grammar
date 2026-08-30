use syn::buffer::Cursor;
use syn::parse::Parser;
use crate::{
    ParseContext, ParseError, ParseResult, PRIO_AGGREGATED, PRIO_LABELED, PRIO_STRUCTURAL,
};

/// Erlaubt das Peeken von spezifischen syn::Tokens auf einem Cursor
pub fn peek_syn<'a, F>(cursor: Cursor<'a>, peek_fn: F) -> bool
where
    F: FnOnce(syn::parse::ParseStream) -> bool,
{
    let stream = cursor.token_stream();
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
pub fn invoke_syn_parser<'a, T: syn::parse::Parse>(mut cursor: Cursor<'a>) -> ParseResult<'a, T> {
    let stream = cursor.token_stream();
    
    // Wir erzeugen einen temporären Syn-Parser
    let parser = |input: syn::parse::ParseStream| {
        let val = input.parse::<T>()?;
        // Zählen der verbleibenden Tokens im Stream
        let remaining = input.cursor().token_stream().into_iter().count();
        // `Parser::parse2` verlangt, dass der GESAMTE Stream verbraucht wird.
        // Ohne dieses Leeren scheitert jeder Aufruf, bei dem T nicht zufaellig
        // bis zum Ende reicht - also bei jeder Sequenz mit mehr als einem Token.
        input.parse::<proc_macro2::TokenStream>()?;
        Ok((val, remaining))
    };
    
    match Parser::parse2(parser, stream.clone()) {
        Ok((val, remaining)) => {
            let total = stream.into_iter().count();
            let consumed = total - remaining;
            
            // Original-Cursor exakt um die Anzahl der verbrauchten Tokens vorschieben
            for _ in 0..consumed {
                if let Some((_, next)) = cursor.token_tree() {
                    cursor = next;
                }
            }
            
            Ok((val, cursor))
        }
        // Span von syn (praezise fuer die Anzeige), Fortschritt vom Eintrittscursor:
        // der steht in einer Sequenz `a b c` beim Scheitern von `c` bereits hinter
        // `a b` und misst die Tiefe damit korrekt.
        Err(e) => Err(ParseError::new(e.span(), e.to_string()).with_cursor(cursor)),
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
    S: FnMut(Cursor<'a>, &mut ParseContext<'a>) -> ParseResult<'a, ()> ,
{
    let mut items = Vec::new();

    // Erstes Element
    match item_parser(cursor, ctx) {
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
        match sep_parser(cursor, &mut sep_ctx) {
            Ok((_, after_sep_cursor)) => {
                let mut item_ctx = sep_ctx.clone();
                
                // Item NACH Separator versuchen
                match item_parser(after_sep_cursor, &mut item_ctx) {
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
                            // Striktes Scheitern: Der Trenner war da, ein Element ist
                            // Pflicht. Der echte Fehler wird ANGEREICHERT, nicht gegen
                            // einen synthetischen ausgetauscht - sonst ginge mit ihm
                            // sein Regelstapel und, wenn er tiefer lag, die
                            // aussagekraeftigere Stelle verloren.
                            return Err(label_missing_item(e, after_sep_cursor, item_name, ctx));
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
            format!("expected at least {} {}s, found {}", min, item_name, items.len()),
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
        match item_parser(cursor, &mut item_ctx) {
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
            format!("expected at least {} {}s, found {}", min, item_name, items.len()),
        )
        .with_priority(PRIO_STRUCTURAL));
    }

    Ok((items, cursor))
}
