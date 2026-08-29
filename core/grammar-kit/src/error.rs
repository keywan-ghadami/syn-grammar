use proc_macro2::Span;
use syn::buffer::Cursor;
use std::fmt;

pub type ParseResult<'a, T> = Result<(T, Cursor<'a>), ParseError>;

#[derive(Debug, Clone)]
pub struct ParseError {
    pub span: Span,
    pub message: String,
    pub priority: u8,
    pub rule_stack: Vec<String>, // Funktionaler Kontext-Speicher!
}

impl ParseError {
    pub fn new(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
            priority: 0,
            rule_stack: Vec::new(),
        }
    }

    pub fn with_priority(mut self, prio: u8) -> Self {
        self.priority = prio;
        self
    }

    pub fn push_rule(&mut self, rule: &str) {
        self.rule_stack.push(rule.to_string());
    }

    pub fn merge(self, other: Self) -> Self {
        if self.priority >= 50 && other.priority < 50 { return self; }
        if other.priority >= 50 && self.priority < 50 { return other; }

        let s_start = self.span.start();
        let o_start = other.span.start();

        if s_start.line > o_start.line || (s_start.line == o_start.line && s_start.column > o_start.column) {
            return self;
        } else if o_start.line > s_start.line || (o_start.line == s_start.line && o_start.column > s_start.column) {
            return other;
        }

        if self.priority > other.priority { self } else { other }
    }
}

/// Action-Bloecke in Grammatiken duerfen weiterhin mit `syn::Error` scheitern.
impl From<syn::Error> for ParseError {
    fn from(e: syn::Error) -> Self {
        ParseError::new(e.span(), e.to_string())
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut msg = self.message.clone();
        let line = self.span.start().line;
        let col = self.span.start().column;
        
        if !msg.contains(&format!("at column {}", col)) {
            msg = format!("{} at column {} (line {})", msg, col, line);
        }

        if !self.rule_stack.is_empty() {
            for rule in &self.rule_stack {
                let suffix = format!("\nin {}", rule);
                if !msg.contains(&suffix) { 
                    msg = format!("{}{}", msg, suffix); 
                }
            }
        }
        write!(f, "{}", msg)
    }
}

impl std::error::Error for ParseError {}
