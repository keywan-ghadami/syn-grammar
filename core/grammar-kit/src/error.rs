use proc_macro2::Span;
use std::fmt;
use syn::buffer::Cursor;

/// The result of a parse step.
///
/// On success the value **and** the cursor behind it - this new cursor *is*
/// the progress. Resetting simply means not using it.
pub type ParseResult<'a, T> = Result<(T, Cursor<'a>), ParseError<'a>>;

// Priority ladder per ADR 13, point 8. Only relevant when two errors are at
// the same position — progress beats priority.

/// Ordinary parse error.
pub const PRIO_NORMAL: u8 = 0;
/// A labelled alternative (`# "…"`) failed at its boundary.
pub const PRIO_LABELED: u8 = 10;
/// Aggregated expectations of several alternatives (`expected one of: …`).
pub const PRIO_AGGREGATED: u8 = 20;
/// `fail(..)` or behind a cut: beats everything else.
pub const PRIO_STRUCTURAL: u8 = 50;

/// A parse error.
///
/// Carries separately what it needs for *display* (`span`) and what it needs
/// for *selection* (`at`) - see `docs/ERROR_HANDLING.md`.
#[derive(Clone)]
pub struct ParseError<'a> {
    /// For DISPLAY: rustc underlines this span in the editor.
    pub span: Span,
    /// For SELECTION: how far the parser got when things went wrong.
    ///
    /// Deliberately not measured via `span.start()`: `Cursor` implements
    /// `PartialOrd` as a pointer comparison within the shared `TokenBuffer` —
    /// O(1) and independent of the compiler version.
    ///
    /// Up to Rust 1.87, `Span::start()` inside a procedural macro also returned
    /// `(0,0)` for every span (proc-macro2, `Span::Compiler` without
    /// `proc_macro_span_location`), so a position comparison would have been
    /// ineffective there. Since 1.88 that is fixed — the cursor metric stays
    /// nonetheless, because it is cheaper and depends on nothing.
    ///
    /// `None` only where no cursor is at hand when creating the error (e.g. when
    /// adopting a foreign `syn::Error`).
    pub at: Option<Cursor<'a>>,
    /// The message text. Never modified during parsing; formatted exactly once
    /// at the end.
    pub message: String,
    /// Rank at the *same* position. See the `PRIO_*` constants.
    pub priority: u8,
    /// Behind a cut (`=>`): the derivation is fixed, resetting is pointless.
    /// Deliberately separate from `priority` — `fail(..)` is high-priority but
    /// not fatal and therefore takes part in the progress comparison.
    pub is_fatal: bool,
    /// The rules in which the error occurred, innermost first. Only for
    /// display - the selection does not use it.
    pub rule_stack: Vec<String>,
}

impl<'a> ParseError<'a> {
    /// Without a cursor — use only when really none is available.
    /// Such errors lose every progress comparison against one with a cursor.
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

    /// The normal case: an error at the position where the parser is.
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

    /// Attaches a cursor to an error that was created without one.
    pub fn with_cursor(mut self, cursor: Cursor<'a>) -> Self {
        self.at = Some(cursor);
        self
    }

    /// Sets the priority (see the `PRIO_*` constants).
    pub fn with_priority(mut self, prio: u8) -> Self {
        self.priority = prio;
        self
    }

    /// Marks the error as having occurred behind a cut.
    pub fn as_fatal(mut self) -> Self {
        self.is_fatal = true;
        self.priority = PRIO_STRUCTURAL;
        self
    }

    /// Appends a rule name to the stack - used on the return path when an
    /// outer rule passes an error outward.
    pub fn push_rule(&mut self, rule: &str) {
        self.rule_stack.push(rule.to_string());
    }

    /// Did `self` get further than `other`?
    ///
    /// `None` if the two cannot be compared — either because one lacks the cursor or
    /// because they come from different `TokenBuffer`s. Within one parse run all
    /// cursors share one buffer, including those from `Cursor::group`; so the case is
    /// defensive, not the normal case.
    fn progress_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self.at, other.at) {
            (Some(a), Some(b)) => a.partial_cmp(&b),
            _ => None,
        }
    }

    /// Picks the more meaningful of two competing errors.
    ///
    /// Order: progress, then priority (`fail`/cut > label > default).
    ///
    /// Progress deliberately comes FIRST - even before a `fail(..)`. Whoever processed
    /// more tokens successfully was closer to the intended derivation; a `fail` located
    /// earlier then describes a branch the parser did not mean at all.
    /// See `error_abstraction_test::test_fail_vs_deep_error`.
    pub fn merge(self, other: Self) -> Self {
        // 1. Progress: whoever got further in the input wins - even against a
        //    `fail(..)` located earlier. Whoever processed more tokens successfully
        //    was closer to the intended derivation.
        match self.progress_cmp(&other) {
            Some(std::cmp::Ordering::Greater) => return self,
            Some(std::cmp::Ordering::Less) => return other,
            Some(std::cmp::Ordering::Equal) => {}
            None => {
                // No cursor comparison possible. An error WITH progress information
                // is more meaningful than one without.
                match (self.at.is_some(), other.at.is_some()) {
                    (true, false) => return self,
                    (false, true) => return other,
                    _ => {}
                }
            }
        }

        // 2. Fatality beats everything else at the SAME position.
        if self.is_fatal != other.is_fatal {
            return if self.is_fatal { self } else { other };
        }

        // 3. Priority: fail > label > default. On a tie the newer one wins.
        if self.priority > other.priority {
            self
        } else {
            other
        }
    }
}

/// Action blocks in grammars may still fail with `syn::Error`.
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

        // Append the position only if it says something. A span without
        // position data reports line 0 — that would be misleading rather than
        // helpful, and rustc underlines the span itself anyway. See ADR 13, point 4.
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

    /// The core test for ADR 13, point 8.
    ///
    /// Selecting the best error must not depend on the span — spans can be
    /// equal without the errors being so (up to Rust 1.87, inside a procedural
    /// macro ALL of them even carried the same position `(0,0)`). Here both
    /// errors deliberately get the same span; they are distinguishable solely
    /// via the cursor.
    #[test]
    fn selection_works_with_identical_spans() {
        let tokens: proc_macro2::TokenStream = "a b c".parse().unwrap();
        let buf = TokenBuffer::new2(tokens);

        let shallow = buf.begin();
        let deep = shallow.token_tree().unwrap().1.token_tree().unwrap().1;

        let same_span = Span::call_site();
        let shallower = ParseError::new(same_span, "shallow").with_cursor(shallow);
        let deeper = ParseError::new(same_span, "deep").with_cursor(deep);

        // The error that got further wins - regardless of the order.
        assert_eq!(shallower.clone().merge(deeper.clone()).message, "deep");
        assert_eq!(deeper.merge(shallower).message, "deep");
    }

    /// Progress beats priority — even a `fail(..)` located earlier.
    /// Whoever processed more tokens was closer to the intended derivation.
    #[test]
    fn progress_beats_fail_priority() {
        let tokens: proc_macro2::TokenStream = "a b c".parse().unwrap();
        let buf = TokenBuffer::new2(tokens);

        let shallow = buf.begin();
        let deep = shallow.token_tree().unwrap().1.token_tree().unwrap().1;

        let shallow_fail =
            ParseError::at_cursor(shallow, "hard fail").with_priority(PRIO_STRUCTURAL);
        let deep_normal = ParseError::at_cursor(deep, "deep");

        assert_eq!(
            shallow_fail.clone().merge(deep_normal.clone()).message,
            "deep"
        );
        assert_eq!(deep_normal.merge(shallow_fail).message, "deep");
    }

    /// At the SAME position the priority then decides in favour of `fail`.
    #[test]
    fn at_same_position_fail_wins() {
        let tokens: proc_macro2::TokenStream = "a b c".parse().unwrap();
        let buf = TokenBuffer::new2(tokens);
        let here = buf.begin();

        let f = ParseError::at_cursor(here, "hard fail").with_priority(PRIO_STRUCTURAL);
        let n = ParseError::at_cursor(here, "normal");

        assert_eq!(f.clone().merge(n.clone()).message, "hard fail");
        assert_eq!(n.merge(f).message, "hard fail");
    }

    /// An error without progress information loses against one with it.
    #[test]
    fn error_with_cursor_beats_error_without() {
        let tokens: proc_macro2::TokenStream = "a b c".parse().unwrap();
        let buf = TokenBuffer::new2(tokens);
        let with_at = ParseError::at_cursor(buf.begin(), "with_at");
        let without_at = ParseError::new(Span::call_site(), "without_at");

        assert_eq!(with_at.clone().merge(without_at.clone()).message, "with_at");
        assert_eq!(without_at.merge(with_at).message, "with_at");
    }

    /// Display of the position (ADR 13, point 4).
    ///
    /// Outside a procedural macro proc-macro2 uses its fallback and returns real lines
    /// (from 1) — the position then belongs in the message. A span without position
    /// data reports line 0 and is suppressed by `Display`; that case cannot be
    /// reproduced here because a `Span::Compiler` cannot be created outside a real
    /// macro. The test therefore records that the information appears in the normal
    /// case — the suppression itself is guarded by the `line != 0` condition in
    /// `Display`.
    #[test]
    fn position_is_printed_in_fallback() {
        let e = ParseError::new(Span::call_site(), "expected `x`");
        assert_eq!(e.to_string(), "expected `x` at column 0 (line 1)");
    }

    /// The rule stack is appended from inside out and deduplicated.
    #[test]
    fn rule_stack_is_appended() {
        let mut e = ParseError::new(Span::call_site(), "expected `x`");
        e.push_rule("inner");
        e.push_rule("outer");
        e.push_rule("outer"); // duplicate is not appended again
        assert_eq!(
            e.to_string(),
            "expected `x` at column 0 (line 1)\nin inner\nin outer"
        );
    }
}
