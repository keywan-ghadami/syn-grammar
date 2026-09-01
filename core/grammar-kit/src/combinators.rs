use crate::{
    gabel, uebernehmen, ParseContext, ParseError, ParseResult, StreamResult, Strom,
    PRIO_AGGREGATED, PRIO_LABELED, PRIO_STRUCTURAL,
};
use syn::buffer::Cursor;

/// Erlaubt das Peeken von spezifischen syn::Tokens auf einem Cursor
pub fn peek_syn<P: syn::parse::Peek>(cursor: Cursor<'_>, token: P) -> bool {
    // Reine Zeigerarithmetik, keine Allokation - genau das, was syns eigenes
    // `ParseStream::peek` tut (`parse.rs`: `T::Token::peek(self.cursor())`).
    //
    // Vorher wurde hier ein Tokenfenster materialisiert und daraus ein
    // kompletter `TokenBuffer` gebaut. Das war doppelt daneben: der Buffer-Bau
    // kostet mehr als der Peek selbst, und ein einzelnes Token kann eine
    // beliebig grosse Delimiter-Gruppe sein - `cursor.token_tree()` liefert
    // `{ ...1000 Tokens... }` als EINEN Tree. Ein "kleines Fenster" war es also
    // nur dem Namen nach.
    //
    // `Peek::Token` und `Token::peek` sind `#[doc(hidden)]`, aber oeffentlich
    // erreichbar. Kein Semver-Versprechen - deshalb genau an dieser einen
    // Stelle gekapselt.
    let _ = token;
    <P::Token as syn::token::Token>::peek(cursor)
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
    prio: u8,
) -> ParseError<'a> {
    if ersetzt_meldung(&e, at) {
        e.message = erwartung_item(at, item_name, ctx);
        e.span = at.span();
    }
    e.priority = e.priority.max(prio);
    e
}

/// Tritt die Erwartung des Elements an die Stelle seiner internen Meldung?
///
/// Nur wenn das Element gar nicht erst vorankam - sonst ist seine eigene Meldung
/// die aussagekraeftigere (ADR 13, Punkt 6).
///
/// Und selbst dann nicht, wenn der Fehler bereits eine eigene Beschriftung
/// traegt: `finish_variants` erzeugt daraus `expected `x`; found unexpected
/// token `y``, was zusaetzlich nennt, was tatsaechlich dastand. Diese Meldung
/// ist reicher als `expected x` und bleibt.
///
/// Ausnahme davon ist das Ende der Eingabe bzw. der Gruppe: dort ist die
/// Angabe, dass der Geltungsbereich endet, wichtiger als jede Aufzaehlung -
/// sonst behauptet die Meldung, es haette etwas dastehen koennen, wo gar nichts
/// mehr kommt (ADR 13, Punkt 3).
fn ersetzt_meldung(e: &ParseError<'_>, at: Cursor<'_>) -> bool {
    e.at == Some(at) && (e.priority < PRIO_LABELED || at.eof())
}

/// Die Erwartung, die an der Stelle eines fehlenden Listenelements gilt.
///
/// Am Ende der Eingabe bzw. der Gruppe wird das mitgesagt - "expected function
/// argument" allein waere dort irrefuehrend (ADR 13, Punkt 3).
fn erwartung_item(at: Cursor<'_>, item_name: &str, ctx: &ParseContext<'_>) -> String {
    if at.eof() {
        format!("{}, expected {}", ctx.end_of_scope_msg(), item_name)
    } else {
        format!("expected {}", item_name)
    }
}

/// Liste aus `item_parser`, getrennt durch `sep_parser`.
///
/// `min` ist die Mindestanzahl, `trailing` erlaubt einen baumelnden Trenner.
/// `item_name` benennt die Elemente in Fehlermeldungen und landet als
/// `"<item_name> <index>"` auf dem lebenden Regelstapel - daher
/// `in function parameter 2`.
///
/// Jeder Versuch laeuft auf einer [`gabel`]; erst der Erfolg wird per
/// [`uebernehmen`] eingespielt. Beim Cursor-Design war Zuruecksetzen gratis
/// (den neuen Cursor einfach nicht benutzen), auf dem Strom kostet es die Gabel
/// - dafuer entfaellt der `TokenBuffer`-Bau je AST-Typ (ADR 15, Stufe 3).
pub fn parse_separated<'a, T, P, S>(
    input: &Strom<'a>,
    ctx: &mut ParseContext<'a>,
    mut item_parser: P,
    mut sep_parser: S,
    min: usize,
    trailing: bool,
    item_name: &str,
) -> StreamResult<'a, Vec<T>>
where
    P: FnMut(&Strom<'a>, &mut ParseContext<'a>) -> StreamResult<'a, T>,
    S: FnMut(&Strom<'a>, &mut ParseContext<'a>) -> StreamResult<'a, ()>,
{
    let mut items = Vec::new();

    // Erstes Element. Der Elementname liegt waehrend des Versuchs auf dem lebenden
    // Stapel - nur so traegt ein TIEF im Element gemerkter Fehler den Listenindex.
    let start = input.cursor();
    let erste_gabel = gabel(input);
    ctx.enter_rule(&format!("{} 1", item_name));
    let erstes = item_parser(&erste_gabel, ctx);
    ctx.exit_rule();
    match erstes {
        Ok(item) => {
            uebernehmen(input, &erste_gabel);
            items.push(item);
        }
        Err(mut e) => {
            if min > 0 {
                // Wird die Meldung ersetzt, sagt auch der interne Regelstapel
                // nichts mehr ueber den Fehler aus - dann zaehlt nur der
                // Listenkontext. Bleibt sie stehen, bleibt er es auch.
                if ersetzt_meldung(&e, start) {
                    e.rule_stack.clear();
                }
                // Der Fehler gehoert zum ersten Element der Liste (ADR 13, Punkt 11).
                e.push_rule(&format!("{} 1", item_name));
                return Err(label_missing_item(
                    e,
                    start,
                    item_name,
                    ctx,
                    PRIO_STRUCTURAL,
                ));
            }
            // Leere Liste ist erlaubt - der Grund, warum kein Element kam, wird
            // aber gemerkt. Sonst bleibt spaeter nur eine generische Meldung.
            //
            // Kam das Element NICHT voran, sagt seine interne Meldung nichts
            // ueber die Liste aus; dann ist "expected <item>" die Antwort, und
            // sie braucht den Rang einer Beschriftung. Ohne den gewinnt an
            // derselben Stelle ein spaeter gemerkter Token-Fehler den
            // Gleichstand - bei `fn f( 123 )` etwa das optionale `","?`, womit
            // aus "expected function argument" ein nichtssagendes
            // "expected `,`" wurde. Siehe ADR 13, Punkt 6.
            //
            // Kam es voran, bleibt alles unangetastet: seine eigene Meldung ist
            // dann die aussagekraeftigere, samt ihrem Regelstapel.
            if ersetzt_meldung(&e, start) {
                e.rule_stack.clear();
            }
            let mut e = label_missing_item(e, start, item_name, ctx, PRIO_LABELED);
            e.push_rule(&format!("{} 1", item_name));
            ctx.record_failure(&e);
            return Ok(items);
        }
    }

    loop {
        let mut sep_ctx = ctx.clone();

        // Separator versuchen - auf einer Gabel, damit der Strom bei Misserfolg
        // VOR dem Trenner stehen bleibt.
        let sep_gabel = gabel(input);
        sep_ctx.enter_rule("separator");
        let sep_res = sep_parser(&sep_gabel, &mut sep_ctx);
        sep_ctx.exit_rule();
        match sep_res {
            Ok(()) => {
                let nach_sep = sep_gabel.cursor();
                let mut item_ctx = sep_ctx.clone();

                // Item NACH Separator versuchen, wieder auf einer eigenen Gabel.
                let item_gabel = gabel(&sep_gabel);
                item_ctx.enter_rule(&format!("{} {}", item_name, items.len() + 1));
                let item_res = item_parser(&item_gabel, &mut item_ctx);
                item_ctx.exit_rule();
                match item_res {
                    Ok(item) => {
                        uebernehmen(input, &item_gabel);
                        items.push(item);
                        *ctx = item_ctx;
                    }
                    Err(mut e) => {
                        // Siehe oben: wird die Meldung ersetzt, traegt der interne
                        // Stapel nichts bei.
                        if ersetzt_meldung(&e, nach_sep) {
                            e.rule_stack.clear();
                        }
                        // Index des VERSUCHTEN Elements, 1-basiert.
                        e.push_rule(&format!("{} {}", item_name, items.len() + 1));
                        if trailing {
                            // Baumelnder Trenner ist erlaubt: er GEHOERT zur Liste und
                            // wird verbraucht. Ohne das blieb er im Strom stehen und
                            // die umgebende Regel scheiterte an ihm.
                            uebernehmen(input, &sep_gabel);
                            *ctx = sep_ctx;
                            ctx.record_failure(&e);
                            break;
                        } else {
                            // Weich zuruecksetzen statt hart scheitern: der Strom
                            // bleibt VOR dem Trenner, damit eine nachfolgende Regel
                            // (etwa ein `","?`) ihn noch verarbeiten kann. Genau darauf
                            // bauen `paren(args:liste? ","?)`-Grammatiken.
                            //
                            // Der Grund wird gemerkt - passt danach doch nichts mehr,
                            // taucht er wieder auf, statt von einer generischen Meldung
                            // ersetzt zu werden. Angereichert wird der ECHTE Fehler,
                            // damit sein Regelstapel und, wenn er tiefer lag, seine
                            // Stelle erhalten bleiben.
                            let markiert =
                                label_missing_item(e, nach_sep, item_name, ctx, PRIO_STRUCTURAL);
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
            input.cursor(),
            format!(
                "expected at least {} {}s, found {}",
                min,
                item_name,
                items.len()
            ),
        )
        .with_priority(PRIO_STRUCTURAL));
    }

    Ok(items)
}

/// Kombinator fuer Wiederholungen ohne Separator.
///
/// Gegenstueck zu [`parse_separated`]. Ein struktureller Fehler (Prioritaet
/// >= 50) bricht die Schleife hart ab, statt sie nur zu beenden.
pub fn parse_repeated<'a, T, P>(
    input: &Strom<'a>,
    ctx: &mut ParseContext<'a>,
    mut item_parser: P,
    min: usize,
    item_name: &str,
) -> StreamResult<'a, Vec<T>>
where
    P: FnMut(&Strom<'a>, &mut ParseContext<'a>) -> StreamResult<'a, T>,
{
    let mut items = Vec::new();

    loop {
        let vorher = input.cursor();
        let item_gabel = gabel(input);
        let mut item_ctx = ctx.clone();
        item_ctx.enter_rule(&format!("{} {}", item_name, items.len() + 1));
        let item_res = item_parser(&item_gabel, &mut item_ctx);
        item_ctx.exit_rule();
        match item_res {
            Ok(item) => {
                // Kein Fortschritt trotz Erfolg -> sonst Endlosschleife.
                if item_gabel.cursor() == vorher {
                    break;
                }
                uebernehmen(input, &item_gabel);
                items.push(item);
                *ctx = item_ctx;
            }
            Err(e) => {
                // Strukturelle/fatale Fehler durchreichen, alles andere beendet
                // die Wiederholung regulaer.
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
            input.cursor(),
            format!(
                "expected at least {} {}s, found {}",
                min,
                item_name,
                items.len()
            ),
        )
        .with_priority(PRIO_STRUCTURAL));
    }

    Ok(items)
}

/// Ein Typ, der aus genau einem Token besteht und deshalb in O(1) direkt vom
/// `Cursor` gelesen werden kann.
///
/// Auch mit [`crate::parse_syn`] (ADR 15, Stufe 3) lohnt das: ein
/// `input.parse::<T>()` laeuft ueber syns Erwartungs- und Fehlermaschinerie,
/// waehrend hier ein Zeigervergleich genuegt. [`crate::schritt`] laesst diese
/// Primitiven auf dem Strom laufen.
///
/// Die Fehlermeldungen sind wortgleich mit denen von syn - mehrere Tests
/// pruefen sie per Substring.
pub trait SingleToken: Sized {
    /// Liest das Token, falls es passt. `None` heisst: passt nicht.
    fn take(cursor: Cursor<'_>) -> Option<(Self, Cursor<'_>)>;
    /// Die Meldung, wenn es nicht passt - wortgleich mit syn.
    fn erwartet() -> &'static str;
}

/// Liest einen [`SingleToken`]-Typ in O(1) vom Cursor.
pub fn take_single<'a, T: SingleToken>(cursor: Cursor<'a>) -> ParseResult<'a, T> {
    match T::take(cursor) {
        Some((wert, next)) => Ok((wert, next)),
        // Am Ende der Eingabe stellt syn seiner Meldung ein
        // "unexpected end of input, " voran. Hier wird das nachgebildet,
        // damit sich die Meldung
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
