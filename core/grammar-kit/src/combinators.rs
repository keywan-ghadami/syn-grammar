use syn::buffer::Cursor;
use syn::parse::Parser;
use crate::{ParseContext, ParseError, ParseResult};

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
        Err(e) => Err(ParseError::new(e.span(), e.to_string())),
    }
}

/// Optionaler Parse-Versuch. Er fängt sanfte Fehler ab und reicht strukturelle Fehler hoch.
pub fn attempt_labeled<'a, T, F>(
    cursor: Cursor<'a>,
    ctx: &mut ParseContext,
    label: Option<&str>,
    parser: F,
) -> ParseResult<'a, Option<T>>
where
    F: FnOnce(Cursor<'a>, &mut ParseContext) -> ParseResult<'a, T>,
{
    let mut fork_ctx = ctx.clone();
    
    match parser(cursor, &mut fork_ctx) {
        Ok((val, next_cursor)) => {
            *ctx = fork_ctx; // Zustand nach erfolgreichem Parse übernehmen
            Ok((Some(val), next_cursor))
        }
        Err(mut e) => {
            // Label applizieren, falls der Fehler exakt am Startpunkt passierte
            if let Some(lbl) = label {
                if e.span.start() == cursor.span().start() {
                    e.message = format!("expected {}", lbl);
                    e.priority = std::cmp::max(e.priority, 10);
                }
            }
            
            // Fatale / Strukturelle Fehler werden ge-bubbled
            if e.priority >= 50 {
                Err(e)
            } else {
                // Reguläres Backtracking: Wir geben den UNVERÄNDERTEN Cursor zurück
                Ok((None, cursor))
            }
        }
    }
}

pub fn parse_separated<'a, T, P, S>(
    mut cursor: Cursor<'a>,
    ctx: &mut ParseContext,
    mut item_parser: P,
    mut sep_parser: S,
    min: usize,
    trailing: bool,
    item_name: &str,
) -> ParseResult<'a, Vec<T>>
where
    P: FnMut(Cursor<'a>, &mut ParseContext) -> ParseResult<'a, T>,
    S: FnMut(Cursor<'a>, &mut ParseContext) -> ParseResult<'a, ()> ,
{
    let mut items = Vec::new();
    let start_span = cursor.span();

    // Erstes Element
    match item_parser(cursor, ctx) {
        Ok((item, next_cursor)) => {
            items.push(item);
            cursor = next_cursor;
        }
        Err(e) => {
            if min > 0 {
                return Err(e.merge(ParseError::new(start_span, format!("expected {}", item_name)).with_priority(50)));
            }
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
                    Err(e) => {
                        if trailing {
                            // Erlaubtes baumelndes Komma: Cursor stoppt nach dem Item davor.
                            break;
                        } else {
                            // Striktes Scheitern: Das Komma war da, das Item fehlt -> Harter Fehler!
                            return Err(e.merge(
                                ParseError::new(after_sep_cursor.span(), format!("expected {}", item_name))
                                .with_priority(50)
                            ));
                        }
                    }
                }
            }
            Err(_) => break, // Kein Komma mehr gefunden, Liste ist fertig
        }
    }

    if items.len() < min {
        return Err(ParseError::new(cursor.span(), format!("expected at least {} {}s", min, item_name)).with_priority(50));
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
    ctx: &mut ParseContext,
    mut item_parser: P,
    min: usize,
    item_name: &str,
) -> ParseResult<'a, Vec<T>>
where
    P: FnMut(Cursor<'a>, &mut ParseContext) -> ParseResult<'a, T>,
{
    let mut items = Vec::new();
    let start_span = cursor.span();

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
                if e.priority >= 50 {
                    return Err(e);
                }
                break;
            }
        }
    }

    if items.len() < min {
        return Err(ParseError::new(
            if items.is_empty() { start_span } else { cursor.span() },
            format!("expected at least {} {}s, found {}", min, item_name, items.len()),
        )
        .with_priority(50));
    }

    Ok((items, cursor))
}
