use proc_macro2::Span;
use std::fmt;
use syn::buffer::Cursor;

/// Das Ergebnis eines Parseschritts.
///
/// Bei Erfolg der Wert **und** der Cursor dahinter - dieser neue Cursor *ist*
/// der Fortschritt. Zuruecksetzen heisst schlicht, ihn nicht zu benutzen.
pub type ParseResult<'a, T> = Result<(T, Cursor<'a>), ParseError<'a>>;

// Prioritätsleiter nach ADR 13, Punkt 8. Nur relevant, wenn zwei Fehler an
// derselben Stelle stehen — Fortschritt schlägt Priorität.

/// Gewöhnlicher Parsefehler.
pub const PRIO_NORMAL: u8 = 0;
/// Eine benannte Alternative (`# "…"`) ist an ihrer Grenze gescheitert.
pub const PRIO_LABELED: u8 = 10;
/// Zusammengefasste Erwartungen mehrerer Alternativen (`expected one of: …`).
pub const PRIO_AGGREGATED: u8 = 20;
/// `fail(..)` oder hinter einem Cut: schlägt alles andere.
pub const PRIO_STRUCTURAL: u8 = 50;

/// Ein Parsefehler.
///
/// Traegt getrennt, was er zur *Anzeige* (`span`) und was er zur *Auswahl*
/// (`at`) braucht - siehe `docs/ERROR_HANDLING.md`.
#[derive(Clone)]
pub struct ParseError<'a> {
    /// Für die ANZEIGE: rustc unterstreicht diesen Span im Editor.
    pub span: Span,
    /// Für die AUSWAHL: wie weit kam der Parser, als es schiefging.
    ///
    /// Bewusst nicht über `span.start()` gemessen. Auf stable Rust liefert
    /// `Span::start()` in einem Prozedurmakro immer `(0,0)`
    /// (proc-macro2 `src/wrapper.rs`, `Span::Compiler` ohne `proc_macro_span_location`),
    /// womit jeder Positionsvergleich wirkungslos wäre. `Cursor` implementiert dagegen
    /// `PartialOrd` als Zeigervergleich im gemeinsamen `TokenBuffer` — O(1) und
    /// unabhängig von der Toolchain.
    ///
    /// `None` nur dort, wo beim Erzeugen kein Cursor zur Hand ist (etwa bei der
    /// Übernahme eines fremden `syn::Error`).
    pub at: Option<Cursor<'a>>,
    /// Der Meldungstext. Waehrend des Parsens nie veraendert; formatiert wird
    /// genau einmal am Ende.
    pub message: String,
    /// Rang bei *gleicher* Stelle. Siehe die `PRIO_*`-Konstanten.
    pub priority: u8,
    /// Hinter einem Cut (`=>`): die Ableitung ist festgelegt, Zurücksetzen ist
    /// sinnlos. Bewusst getrennt von `priority` — `fail(..)` ist hochprior, aber
    /// nicht fatal und nimmt deshalb am Fortschrittsvergleich teil.
    pub is_fatal: bool,
    /// Die Regeln, in denen der Fehler auftrat, innerste zuerst. Nur fuer die
    /// Anzeige - die Auswahl benutzt ihn nicht.
    pub rule_stack: Vec<String>,
}

impl<'a> ParseError<'a> {
    /// Ohne Cursor — nur verwenden, wenn wirklich keiner verfügbar ist.
    /// Solche Fehler verlieren jeden Fortschrittsvergleich gegen einen mit Cursor.
    pub fn new(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            at: None,
            message: message.into(),
            priority: 0,
            is_fatal: false,
            rule_stack: Vec::new(),
        }
    }

    /// Der Normalfall: Fehler an der Stelle, an der der Parser steht.
    pub fn at_cursor(cursor: Cursor<'a>, message: impl Into<String>) -> Self {
        Self {
            span: cursor.span(),
            at: Some(cursor),
            message: message.into(),
            priority: 0,
            is_fatal: false,
            rule_stack: Vec::new(),
        }
    }

    /// Hängt einen Cursor an einen Fehler, der ohne erzeugt wurde.
    pub fn with_cursor(mut self, cursor: Cursor<'a>) -> Self {
        self.at = Some(cursor);
        self
    }

    /// Setzt die Prioritaet (siehe die `PRIO_*`-Konstanten).
    pub fn with_priority(mut self, prio: u8) -> Self {
        self.priority = prio;
        self
    }

    /// Markiert den Fehler als hinter einem Cut entstanden.
    pub fn as_fatal(mut self) -> Self {
        self.is_fatal = true;
        self.priority = PRIO_STRUCTURAL;
        self
    }

    /// Haengt einen Regelnamen an den Stapel an - benutzt auf dem
    /// Rueckgabepfad, wenn eine aeussere Regel einen Fehler herausreicht.
    pub fn push_rule(&mut self, rule: &str) {
        self.rule_stack.push(rule.to_string());
    }

    /// Kam `self` weiter als `other`?
    ///
    /// `None`, wenn sich die beiden nicht vergleichen lassen — entweder weil einem der
    /// Cursor fehlt oder weil sie aus verschiedenen `TokenBuffer`n stammen. Innerhalb
    /// eines Parse-Laufs teilen sich alle Cursor einen Buffer, auch die aus
    /// `Cursor::group`; der Fall ist also defensiv, nicht der Normalfall.
    fn progress_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self.at, other.at) {
            (Some(a), Some(b)) => a.partial_cmp(&b),
            _ => None,
        }
    }

    /// Wählt aus zwei konkurrierenden Fehlern den aussagekräftigeren.
    ///
    /// Reihenfolge: Fortschritt, dann Priorität (`fail`/Cut > Label > Standard).
    ///
    /// Fortschritt kommt bewusst ZUERST - auch vor einem `fail(..)`. Wer mehr Tokens
    /// erfolgreich verarbeitet hat, war näher an der gemeinten Ableitung; ein früher
    /// stehendes `fail` beschreibt dann einen Zweig, den der Parser gar nicht meinte.
    /// Siehe `error_abstraction_test::test_fail_vs_deep_error`.
    pub fn merge(self, other: Self) -> Self {
        // 1. Fortschritt: wer weiter im Input kam, gewinnt - auch gegen einen
        //    `fail(..)`, das frueher stand. Wer mehr Tokens erfolgreich verarbeitet
        //    hat, war naeher an der gemeinten Ableitung.
        match self.progress_cmp(&other) {
            Some(std::cmp::Ordering::Greater) => return self,
            Some(std::cmp::Ordering::Less) => return other,
            Some(std::cmp::Ordering::Equal) => {}
            None => {
                // Kein Cursorvergleich möglich. Ein Fehler MIT Fortschrittsangabe ist
                // aussagekräftiger als einer ohne.
                match (self.at.is_some(), other.at.is_some()) {
                    (true, false) => return self,
                    (false, true) => return other,
                    _ => {}
                }
            }
        }

        // 2. Fatalität schlägt bei GLEICHER Stelle alles andere.
        if self.is_fatal != other.is_fatal {
            return if self.is_fatal { self } else { other };
        }

        // 3. Priorität: fail > Label > Standard. Bei Gleichstand gewinnt der neuere.
        if self.priority > other.priority {
            self
        } else {
            other
        }
    }
}

/// Action-Bloecke in Grammatiken duerfen weiterhin mit `syn::Error` scheitern.
impl<'a> From<syn::Error> for ParseError<'a> {
    fn from(e: syn::Error) -> Self {
        ParseError::new(e.span(), e.to_string())
    }
}

impl<'a> fmt::Debug for ParseError<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ParseError")
            .field("message", &self.message)
            .field("priority", &self.priority)
            .field("rule_stack", &self.rule_stack)
            .field("has_cursor", &self.at.is_some())
            .finish()
    }
}

impl<'a> fmt::Display for ParseError<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut msg = self.message.clone();
        let start = self.span.start();

        // Position nur anhängen, wenn sie etwas aussagt. In einem Prozedurmakro auf
        // stable Rust ist sie (0,0) und damit irreführend statt hilfreich; dort
        // unterstreicht rustc den Span ohnehin selbst. Siehe ADR 13, Punkt 4.
        if start.line != 0 && !msg.contains("at column ") {
            msg = format!("{} at column {} (line {})", msg, start.column, start.line);
        }

        for rule in &self.rule_stack {
            let suffix = format!("\nin {}", rule);
            if !msg.contains(&suffix) {
                msg = format!("{}{}", msg, suffix);
            }
        }
        write!(f, "{}", msg)
    }
}

impl<'a> std::error::Error for ParseError<'a> {}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::buffer::TokenBuffer;

    /// Der Kerntest zu ADR 13, Punkt 8.
    ///
    /// In einem Prozedurmakro auf stable Rust tragen ALLE Spans dieselbe Position
    /// `(0,0)`. Die Auswahl des besten Fehlers muss trotzdem funktionieren — sie darf
    /// deshalb nicht am Span hängen. Hier bekommen beide Fehler bewusst denselben
    /// Span; unterscheidbar sind sie allein über den Cursor.
    #[test]
    fn auswahl_funktioniert_bei_identischen_spans() {
        let tokens: proc_macro2::TokenStream = "a b c".parse().unwrap();
        let buf = TokenBuffer::new2(tokens);

        let flach = buf.begin();
        let tief = flach.token_tree().unwrap().1.token_tree().unwrap().1;

        let gleicher_span = Span::call_site();
        let flacher = ParseError::new(gleicher_span, "flach").with_cursor(flach);
        let tieferer = ParseError::new(gleicher_span, "tief").with_cursor(tief);

        // Der weiter gekommene Fehler gewinnt - unabhaengig von der Reihenfolge.
        assert_eq!(flacher.clone().merge(tieferer.clone()).message, "tief");
        assert_eq!(tieferer.merge(flacher).message, "tief");
    }

    /// Fortschritt schlägt Priorität — auch ein `fail(..)`, das früher steht.
    /// Wer mehr Tokens verarbeitet hat, war näher an der gemeinten Ableitung.
    #[test]
    fn fortschritt_schlaegt_fail_prioritaet() {
        let tokens: proc_macro2::TokenStream = "a b c".parse().unwrap();
        let buf = TokenBuffer::new2(tokens);

        let flach = buf.begin();
        let tief = flach.token_tree().unwrap().1.token_tree().unwrap().1;

        let flach_fail = ParseError::at_cursor(flach, "hard fail").with_priority(PRIO_STRUCTURAL);
        let tief_normal = ParseError::at_cursor(tief, "tief");

        assert_eq!(
            flach_fail.clone().merge(tief_normal.clone()).message,
            "tief"
        );
        assert_eq!(tief_normal.merge(flach_fail).message, "tief");
    }

    /// An DERSELBEN Stelle entscheidet dann die Priorität zugunsten von `fail`.
    #[test]
    fn bei_gleicher_stelle_gewinnt_fail() {
        let tokens: proc_macro2::TokenStream = "a b c".parse().unwrap();
        let buf = TokenBuffer::new2(tokens);
        let hier = buf.begin();

        let f = ParseError::at_cursor(hier, "hard fail").with_priority(PRIO_STRUCTURAL);
        let n = ParseError::at_cursor(hier, "normal");

        assert_eq!(f.clone().merge(n.clone()).message, "hard fail");
        assert_eq!(n.merge(f).message, "hard fail");
    }

    /// Ein Fehler ohne Fortschrittsangabe verliert gegen einen mit.
    #[test]
    fn fehler_mit_cursor_schlaegt_fehler_ohne() {
        let tokens: proc_macro2::TokenStream = "a b c".parse().unwrap();
        let buf = TokenBuffer::new2(tokens);
        let mit = ParseError::at_cursor(buf.begin(), "mit");
        let ohne = ParseError::new(Span::call_site(), "ohne");

        assert_eq!(mit.clone().merge(ohne.clone()).message, "mit");
        assert_eq!(ohne.merge(mit).message, "mit");
    }

    /// Anzeige der Position (ADR 13, Punkt 4).
    ///
    /// Ausserhalb eines Prozedurmakros benutzt proc-macro2 seinen Fallback und liefert
    /// echte Zeilen (ab 1) — die Position gehoert dann in die Meldung. Im Prozedurmakro
    /// auf stable ist sie `(0,0)` und wird von `Display` unterdrueckt; dieser Fall
    /// laesst sich hier nicht nachstellen, weil sich ein `Span::Compiler` ausserhalb
    /// eines echten Makros nicht erzeugen laesst. Der Test haelt deshalb fest, dass die
    /// Angabe im Normalfall erscheint — die Unterdrueckung selbst sichert die
    /// `line != 0`-Bedingung in `Display`.
    #[test]
    fn position_wird_im_fallback_gedruckt() {
        let e = ParseError::new(Span::call_site(), "expected `x`");
        assert_eq!(e.to_string(), "expected `x` at column 0 (line 1)");
    }

    /// Der Regelstapel haengt von innen nach aussen an und dedupliziert.
    #[test]
    fn regelstapel_wird_angehaengt() {
        let mut e = ParseError::new(Span::call_site(), "expected `x`");
        e.push_rule("inner");
        e.push_rule("outer");
        e.push_rule("outer"); // Duplikat wird nicht erneut angehaengt
        assert_eq!(
            e.to_string(),
            "expected `x` at column 0 (line 1)\nin inner\nin outer"
        );
    }
}
