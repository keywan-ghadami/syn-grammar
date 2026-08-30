use crate::ParseError;
use proc_macro2::Span;
use std::collections::HashSet;

#[derive(Clone, Default)]
pub struct ScopeStack {
    pub scopes: Vec<HashSet<String>>,
}

impl ScopeStack {
    pub fn new() -> Self { Self { scopes: vec![HashSet::new()] } }
    pub fn enter_scope(&mut self) { self.scopes.push(HashSet::new()); }
    pub fn exit_scope(&mut self) { if self.scopes.len() > 1 { self.scopes.pop(); } }
    
    pub fn define(&mut self, name: impl Into<String>) {
        if let Some(scope) = self.scopes.last_mut() { scope.insert(name.into()); }
    }
    
    pub fn is_defined(&self, name: &str) -> bool {
        self.scopes.iter().rev().any(|scope| scope.contains(name))
    }
}

/// Schlanker State für Variablen, Whitespace-Modus und Span-Tracking.
/// Wird beim Backtracking geklont (billig, da meist winzig).
#[derive(Clone)]
pub struct ParseContext<'a> {
    /// Die weiteste Stelle, an der das Parsen jemals gescheitert ist.
    ///
    /// Ein rein funktionales Modell verliert jeden Fehler, der von einem
    /// *erfolgreichen* Zuruecksetzen ueberdeckt wird: `separated` mit `min=0`, ein
    /// gescheitertes `?` oder `*` liefern `Ok`, und ein `Ok` traegt keinen Fehler.
    /// Genau dann bleibt am Ende nur eine flache, generische Meldung uebrig.
    /// Dieses Feld ist der Kanal dafuer - es wird beim Zuruecksetzen NICHT
    /// verworfen, sondern hochgereicht.
    pub furthest: Option<ParseError<'a>>,
    pub scopes: ScopeStack,
    pub mode_stack: Vec<bool>,
    pub last_span: Option<Span>, // Wichtig für den Lexical-Mode!
    /// Wie tief stehen wir in Delimiter-Gruppen (`paren(..)`, `{..}`, `[..]`)?
    ///
    /// `Cursor::eof()` bezieht sich auf den *Scope*, meldet am Ende einer Gruppe
    /// also dasselbe wie am Ende der Eingabe. Für die Meldung ist der Unterschied
    /// aber wesentlich: "unexpected end of group" gegen "unexpected end of input".
    /// Zur Laufzeit sind beide nicht unterscheidbar, der Codegen weiss es jedoch.
    pub group_depth: usize,
}

impl<'a> ParseContext<'a> {
    pub fn new() -> Self {
        Self {
            scopes: ScopeStack::new(),
            mode_stack: Vec::new(),
            last_span: None,
            group_depth: 0,
            furthest: None,
        }
    }

    /// Merkt sich einen Fehler, auch wenn der Parser danach erfolgreich weitermacht.
    pub fn record_failure(&mut self, e: &ParseError<'a>) {
        self.furthest = Some(match self.furthest.take() {
            Some(bisher) => bisher.merge(e.clone()),
            None => e.clone(),
        });
    }

    /// Uebernimmt die Merkstelle eines verworfenen Kontext-Klons.
    ///
    /// Ohne das ginge der Fehler genau in den Faellen verloren, um die es geht:
    /// beim Backtracking wird der Klon weggeworfen.
    pub fn absorb(&mut self, other: &ParseContext<'a>) {
        if let Some(e) = &other.furthest {
            self.record_failure(e);
        }
    }

    /// Waehlt zwischen dem zurueckgegebenen Fehler und der gemerkten Stelle.
    pub fn best_error(&self, returned: ParseError<'a>) -> ParseError<'a> {
        match &self.furthest {
            Some(f) => returned.merge(f.clone()),
            None => returned,
        }
    }

    pub fn enter_group(&mut self) { self.group_depth += 1; }
    pub fn exit_group(&mut self) { self.group_depth = self.group_depth.saturating_sub(1); }
    /// Beschreibt das Ende des aktuellen Scopes so, wie es in einer Meldung stehen soll.
    pub fn end_of_scope_msg(&self) -> &'static str {
        if self.group_depth > 0 {
            "unexpected end of group"
        } else {
            "unexpected end of input"
        }
    }

    pub fn enter_lexical(&mut self) { self.mode_stack.push(true); }
    pub fn enter_spaced(&mut self) { self.mode_stack.push(false); }
    pub fn exit_mode(&mut self) { self.mode_stack.pop(); }
    pub fn is_lexical(&self) -> bool { *self.mode_stack.last().unwrap_or(&false) }

    /// Zeichnet den Span auf und wirft einen Fehler, wenn im Lexical-Mode 
    /// fälschlicherweise Leerzeichen zwischen den Tokens stehen.
    pub fn record_span(&mut self, span: Span) -> syn::Result<()> {
        if self.is_lexical() {
            if let Some(last) = self.last_span {
                // Wenn das Ende des letzten Tokens nicht der Anfang des neuen ist, gibt es Whitespace
                if last.end() != span.start() {
                    return Err(syn::Error::new(span, "expected no whitespace"));
                }
            }
        }
        self.last_span = Some(span);
        Ok(())
    }

    pub fn check_whitespace(&self, next_span: Span) -> bool {
        if let Some(last) = self.last_span { 
            last.end() != next_span.start() 
        } else { 
            true 
        }
    }
}

impl<'a> Default for ParseContext<'a> {
    fn default() -> Self { Self::new() }
}
