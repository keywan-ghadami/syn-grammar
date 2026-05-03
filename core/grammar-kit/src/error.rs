use proc_macro2::Span;
use syn::buffer::Cursor;

/// Die fundamentale Parser-Monade.
/// Gibt bei Erfolg den generierten AST-Knoten und den GESTEPPTEN Cursor zurück.
pub type ParseResult<'a, T> = Result<(T, Cursor<'a>), ParseError>;

#[derive(Debug, Clone)]
pub struct ParseError {
    pub span: Span,
    pub message: String,
    pub priority: u8,
}

impl ParseError {
    pub fn new(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
            priority: 0,
        }
    }

    pub fn with_priority(mut self, prio: u8) -> Self {
        self.priority = prio;
        self
    }

    /// Die funktionale Alternative zu `take_best_error`.
    /// Führt deterministisch zwei gescheiterte ODER-Zweige zusammen.
    pub fn merge(self, other: Self) -> Self {
        // 1. Fatale/Strukturelle Fehler (wie fehlende Kommas oder fail!) gewinnen immer
        if self.priority >= 50 && other.priority < 50 { return self; }
        if other.priority >= 50 && self.priority < 50 { return other; }

        // 2. Tiefe im Stream (Progress = Preservation)
        let s_start = self.span.start();
        let o_start = other.span.start();

        if s_start.line > o_start.line || (s_start.line == o_start.line && s_start.column > o_start.column) {
            return self;
        } else if o_start.line > s_start.line || (o_start.line == s_start.line && o_start.column > s_start.column) {
            return other;
        }

        // 3. Gleiche Position: Priorität entscheidet.
        if self.priority > other.priority {
            self
        } else {
            other
        }
    }
}
