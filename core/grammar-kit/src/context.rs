use crate::ParseError;
use proc_macro2::Span;
use std::collections::HashSet;

/// Nested scopes for grammars that manage identifiers themselves
/// (e.g. to recognise already declared names).
#[derive(Clone, Default)]
pub struct ScopeStack {
    /// The levels, outermost first.
    pub scopes: Vec<HashSet<String>>,
}

impl ScopeStack {
    /// A stack with one empty outermost level.
    pub fn new() -> Self {
        Self {
            scopes: vec![HashSet::new()],
        }
    }
    /// Opens a new inner level.
    pub fn enter_scope(&mut self) {
        self.scopes.push(HashSet::new());
    }
    /// Closes the innermost level; the outermost always remains.
    pub fn exit_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    /// Registers `name` in the innermost level.
    pub fn define(&mut self, name: impl Into<String>) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.into());
        }
    }

    /// Is `name` known in any level? Searches from inside out.
    pub fn is_defined(&self, name: &str) -> bool {
        self.scopes.iter().rev().any(|scope| scope.contains(name))
    }
}

/// Lean state for variables, whitespace mode and span tracking.
/// Cloned on backtracking (cheap, since usually tiny).
#[derive(Clone)]
pub struct ParseContext<'a> {
    /// The furthest position at which parsing ever failed.
    ///
    /// A purely functional model loses every error that is covered by a
    /// *successful* reset: `separated` with `min=0`, a failed `?` or `*` return
    /// `Ok`, and an `Ok` carries no error. Exactly then only a flat, generic
    /// message remains at the end. This field is the channel for that - on reset
    /// it is NOT discarded but passed upward.
    pub furthest: Option<ParseError<'a>>,
    /// Scopes for grammars that manage identifiers themselves.
    pub scopes: ScopeStack,
    /// The stack of whitespace modes: `true` = lexical (`lex(..)`, no
    /// whitespace allowed), `false` = `spaced(..)`.
    pub mode_stack: Vec<bool>,
    /// The span of the most recently read token - the basis of the
    /// adjacency check in lexical mode.
    pub last_span: Option<Span>,
    /// How deep are we in delimiter groups (`paren(..)`, `{..}`, `[..]`)?
    ///
    /// `Cursor::eof()` refers to the *scope*, so at the end of a group it reports
    /// the same as at the end of the input. For the message the difference is
    /// essential though: "unexpected end of group" versus "unexpected end of input".
    /// At runtime the two are indistinguishable, but the codegen knows.
    pub group_depth: usize,
    /// The rules the parser is CURRENTLY in, outermost first.
    ///
    /// An error that is passed outward collects its rule names on the way itself
    /// (`push_rule`). An error that is covered by a successful reset and merely
    /// recorded never takes that path - for it a snapshot is taken here.
    pub rule_stack: Vec<String>,
}

impl<'a> ParseContext<'a> {
    /// A fresh context for a parse run.
    pub fn new() -> Self {
        Self {
            scopes: ScopeStack::new(),
            mode_stack: Vec::new(),
            last_span: None,
            group_depth: 0,
            furthest: None,
            rule_stack: Vec::new(),
        }
    }

    /// Pushes `name` onto the live rule stack. Wraps the rule body in the generated
    /// code and is paired with [`exit_rule`](Self::exit_rule).
    pub fn enter_rule(&mut self, name: &str) {
        self.rule_stack.push(name.to_string());
    }

    /// Pops the innermost rule name off the live stack again.
    pub fn exit_rule(&mut self) {
        self.rule_stack.pop();
    }

    /// Records an error even if the parser continues successfully afterwards.
    ///
    /// In doing so it receives the rules the parser is currently in - from inside
    /// out, matching what is already attached to it. Without that, a covered error
    /// carries only the context it collected up to being discarded.
    pub fn record_failure(&mut self, e: &ParseError<'a>) {
        let mut e = e.clone();
        for name in self.rule_stack.iter().rev() {
            if !e.rule_stack.iter().any(|r| r == name) {
                e.rule_stack.push(name.clone());
            }
        }
        self.furthest = Some(match self.furthest.take() {
            Some(bisher) => bisher.merge(e),
            None => e,
        });
    }

    /// Adopts the recorded error of a discarded context clone.
    ///
    /// Without this the error would be lost in exactly the cases that matter:
    /// on backtracking the clone is thrown away.
    pub fn absorb(&mut self, other: &ParseContext<'a>) {
        if let Some(e) = &other.furthest {
            self.record_failure(e);
        }
    }

    /// Chooses between the returned error and the recorded one.
    pub fn best_error(&self, returned: ParseError<'a>) -> ParseError<'a> {
        match &self.furthest {
            Some(f) => returned.merge(f.clone()),
            None => returned,
        }
    }

    /// Enters a delimiter group (`paren(..)`, `{..}`, `[..]`).
    pub fn enter_group(&mut self) {
        self.group_depth += 1;
    }
    /// Leaves a delimiter group.
    pub fn exit_group(&mut self) {
        self.group_depth = self.group_depth.saturating_sub(1);
    }
    /// Describes the end of the current scope the way it should appear in a message.
    pub fn end_of_scope_msg(&self) -> &'static str {
        if self.group_depth > 0 {
            "unexpected end of group"
        } else {
            "unexpected end of input"
        }
    }

    /// Enters a `lex(..)` block: no whitespace is allowed between the tokens.
    pub fn enter_lexical(&mut self) {
        self.mode_stack.push(true);
    }
    /// Enters a `spaced(..)` block: whitespace is allowed again.
    pub fn enter_spaced(&mut self) {
        self.mode_stack.push(false);
    }
    /// Leaves the innermost whitespace mode.
    pub fn exit_mode(&mut self) {
        self.mode_stack.pop();
    }
    /// Is lexical mode currently active?
    pub fn is_lexical(&self) -> bool {
        *self.mode_stack.last().unwrap_or(&false)
    }

    /// Records the span and raises an error if, in lexical mode, there is
    /// whitespace between the tokens where there must not be.
    pub fn record_span(&mut self, span: Span) -> syn::Result<()> {
        if self.is_lexical() {
            if let Some(last) = self.last_span {
                // If the end of the last token is not the start of the new one, there is whitespace
                if last.end() != span.start() {
                    return Err(syn::Error::new(span, "expected no whitespace"));
                }
            }
        }
        self.last_span = Some(span);
        Ok(())
    }

    /// Is there actually whitespace before `next_span`? Backs the
    /// `whitespace` builtin.
    pub fn check_whitespace(&self, next_span: Span) -> bool {
        if let Some(last) = self.last_span {
            last.end() != next_span.start()
        } else {
            true
        }
    }
}

impl<'a> Default for ParseContext<'a> {
    fn default() -> Self {
        Self::new()
    }
}
