#![doc = include_str!("../README.md")]

#[cfg(feature = "syn")]
use proc_macro2::Span;
use std::collections::HashSet;
#[cfg(feature = "syn")]
use syn::parse::discouraged::Speculative;
#[cfg(feature = "syn")]
use syn::parse::ParseStream;
#[cfg(feature = "syn")]
use syn::Result;

#[cfg(feature = "testing")]
pub mod testing;

pub mod macros;
pub mod test_macros;

/// Generic symbol table that tracks variable definitions in nested scopes.
#[derive(Clone, Default)]
pub struct ScopeStack {
    scopes: Vec<HashSet<String>>,
}

impl ScopeStack {
    pub fn new() -> Self {
        Self {
            scopes: vec![HashSet::new()],
        }
    }

    pub fn enter_scope(&mut self) {
        self.scopes.push(HashSet::new());
    }

    pub fn exit_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    pub fn define(&mut self, name: impl Into<String>) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.into());
        }
    }

    pub fn is_defined(&self, name: &str) -> bool {
        for scope in self.scopes.iter().rev() {
            if scope.contains(name) {
                return true;
            }
        }
        false
    }

    pub fn scopes(&self) -> &Vec<HashSet<String>> {
        &self.scopes
    }
}

#[cfg(all(feature = "rt", feature = "syn"))]
#[derive(Clone, Debug)]
struct ErrorState {
    err: syn::Error,
    rule_stack: Vec<String>,
    start_span: Span,
    priority: u8,
    is_fatal: bool,
    label: Option<String>,
}

/// Holds the state for backtracking and error reporting.
/// This must be passed mutably through the parsing chain.
#[cfg(feature = "rt")]
#[derive(Clone)]
pub struct ParseContext {
    is_fatal: bool,
    #[cfg(feature = "syn")]
    best_error: Option<ErrorState>,
    pub scopes: ScopeStack,
    rule_stack: Vec<String>,
    #[cfg(feature = "syn")]
    pub last_span: Option<Span>,
    fail_triggered: bool,
    suppress_label: bool,
}

#[cfg(feature = "rt")]
impl ParseContext {
    pub fn new() -> Self {
        Self {
            is_fatal: false,
            #[cfg(feature = "syn")]
            best_error: None,
            scopes: ScopeStack::new(),
            rule_stack: Vec::new(),
            #[cfg(feature = "syn")]
            last_span: None,
            fail_triggered: false,
            suppress_label: false,
        }
    }

    pub fn set_fatal(&mut self, fatal: bool) {
        self.is_fatal = fatal;
    }

    pub fn check_fatal(&self) -> bool {
        self.is_fatal
    }

    pub fn trigger_fail(&mut self) {
        self.fail_triggered = true;
    }

    pub fn suppress_label(&mut self) {
        self.suppress_label = true;
    }

    /// **High-Level Abstraction**
    /// Marks the current parse path as definitive. Any subsequent errors
    /// will be treated as fatal, preventing backtracking to other alternatives.
    /// Used by the Cut operator (`=>`).
    pub fn commit(&mut self) {
        self.is_fatal = true;
    }

    /// **High-Level Abstraction**
    /// Immediately raises a specific error, overriding any "better" errors found so far.
    /// This is used for semantic checks (`fail "msg"`) or structural validation (wrong separator).
    ///
    /// Clears previous errors, sets priority to High (2), and suppresses default labeling.
    #[cfg(feature = "syn")]
    pub fn raise_failure<T>(&mut self, msg: impl std::fmt::Display, span: Span) -> Result<T> {
        // Trigger high priority handling
        self.fail_triggered = true;

        // Don't auto-label this error (e.g. don't say "expected identifier" if we explicitly say "number too big")
        self.suppress_label = true;

        Err(syn::Error::new(span, msg))
    }

    pub fn enter_rule(&mut self, name: &str) {
        #[cfg(feature = "trace")]
        eprintln!("[TRACE] enter_rule: {}", name);
        self.rule_stack.push(name.to_string());
    }

    pub fn exit_rule(&mut self) {
        let _name = self.rule_stack.pop();
        #[cfg(feature = "trace")]
        if let Some(n) = _name {
            eprintln!("[TRACE] exit_rule: {}", n);
        }
    }

    /// Records an error if it is "better" than the current best error.
    #[cfg(feature = "syn")]
    pub fn record_error(
        &mut self,
        err: syn::Error,
        _attempt_span: Span,
        label: Option<String>,
        mut priority: u8,
    ) {
        // If fail was triggered, bump priority to at least 2
        if self.fail_triggered {
            priority = std::cmp::max(priority, 2);
        }
        self.fail_triggered = false; // Reset after consuming

        #[cfg(feature = "trace")]
        eprintln!(
            "[TRACE] record_error: '{}', priority: {}, label: {:?}",
            err, priority, label
        );

        // We use the error's actual location for comparison
        let error_span = err.span();

        let new_error_state = ErrorState {
            err,
            rule_stack: self.rule_stack.clone(),
            start_span: error_span,
            priority,
            is_fatal: self.is_fatal,
            label,
        };

        match &mut self.best_error {
            None => {
                self.best_error = Some(new_error_state);
            }
            Some(existing) => {
                // 1. Fatality: If the new error is fatal, it wins immediately.
                if new_error_state.is_fatal && !existing.is_fatal {
                    self.best_error = Some(new_error_state);
                    return;
                }
                if existing.is_fatal && !new_error_state.is_fatal {
                    return;
                }

                // 2. Location (Progress)
                let new_start = new_error_state.start_span.start();
                let old_start = existing.start_span.start();

                let is_deeper = new_start.line > old_start.line
                    || (new_start.line == old_start.line && new_start.column > old_start.column);

                let is_shallower = old_start.line > new_start.line
                    || (old_start.line == new_start.line && old_start.column > new_start.column);

                if is_deeper {
                    self.best_error = Some(new_error_state);
                    return;
                } else if is_shallower {
                    return;
                }

                // 3. Priority
                if new_error_state.priority > existing.priority {
                    self.best_error = Some(new_error_state);
                    return;
                } else if existing.priority > new_error_state.priority {
                    return;
                }

                // 4. Context specificity
                // Prefer deeper rule stack or one with label
                if new_error_state.rule_stack.len() > existing.rule_stack.len()
                    || (new_error_state.label.is_some() && existing.label.is_none())
                {
                    self.best_error = Some(new_error_state);
                } else if new_error_state.rule_stack.len() == existing.rule_stack.len() {
                    // Tie-breaker: longer message length (more info)
                    if new_error_state.err.to_string().len() >= existing.err.to_string().len() {
                        self.best_error = Some(new_error_state);
                    }
                }
            }
        }
    }

    #[cfg(feature = "syn")]
    pub fn take_best_error(&mut self) -> Option<syn::Error> {
        let best = self.best_error.take()?;

        let mut msg = best.err.to_string();

        // Apply label if present
        if let Some(label) = &best.label {
            // If the message is generic (e.g. from an empty Result), use the label.
            // If the message is already specific (e.g. "expected one of..."), keep it.
            // Heuristic: If it starts with "expected", we assume it's already formatted.
            // But sometimes the label IS what we want.
            // For now, simple override if not already containing "expected".
            if !msg.contains("expected") {
                msg = format!("expected {}", label);
            }
        }

        // Apply rule stack
        if !best.rule_stack.is_empty() {
            // Apply prefixes in reverse order (stack order)
            // But be careful not to double-apply if the error message already has them.
            for rule in best.rule_stack.iter().rev() {
                let prefix = format!("in rule `{}`: ", rule);

                // Robust check: Does the message start with this prefix?
                // Or does it start with "in rule `X`: " where X is something else?
                // We want to prepend ONLY if it's missing.

                if !msg.starts_with(&prefix) {
                    msg = format!("{}{}", prefix, msg);
                }
            }
        }

        Some(syn::Error::new(best.start_span, msg))
    }

    /// Determines if the current best error is "significant enough" to stop
    /// aggregating shallow alternative errors.
    #[cfg(feature = "syn")]
    pub fn stop_aggregation(&self, current_span: Span) -> bool {
        if let Some(e) = &self.best_error {
            // 1. Fatal errors always stop aggregation
            if e.is_fatal {
                return true;
            }

            // 2. Explicit failures (priority > 1) stop aggregation.
            // Priority 1 (labeled shallow error) is considered insignificant for aggregation.
            if e.priority > 1 {
                return true;
            }

            // 3. Deep errors (progress made beyond current start) stop aggregation.
            let e_start = e.start_span.start();
            let c_start = current_span.start();

            if e_start.line > c_start.line
                || (e_start.line == c_start.line && e_start.column > c_start.column)
            {
                return true;
            }
        }
        false
    }

    #[cfg(feature = "syn")]
    pub fn is_best_error_deep(&self) -> bool {
        // Compatibility: check if priority > 0 (fail or label)
        self.best_error
            .as_ref()
            .map(|e| e.priority > 0)
            .unwrap_or(false)
    }

    // --- Span Tracking ---

    #[cfg(feature = "syn")]
    pub fn record_span(&mut self, span: Span) {
        self.last_span = Some(span);
    }

    #[cfg(feature = "syn")]
    pub fn check_whitespace(&self, next_span: Span) -> bool {
        if let Some(last) = self.last_span {
            // Check if they are NOT adjacent (end != start)
            last.end() != next_span.start()
        } else {
            // No previous token? Treat as valid (start of file)
            true
        }
    }

    // --- Symbol Table Methods ---

    pub fn enter_scope(&mut self) {
        self.scopes.enter_scope();
    }

    pub fn exit_scope(&mut self) {
        self.scopes.exit_scope();
    }

    pub fn define(&mut self, name: impl Into<String>) {
        self.scopes.define(name);
    }

    pub fn is_defined(&self, name: &str) -> bool {
        self.scopes.is_defined(name)
    }

    // --- Inspection Methods ---

    pub fn scopes(&self) -> &Vec<HashSet<String>> {
        self.scopes.scopes()
    }

    pub fn rule_stack(&self) -> &Vec<String> {
        &self.rule_stack
    }
}

#[cfg(feature = "rt")]
impl Default for ParseContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Encapsulates a speculative parse attempt.
/// Requires passing the ParseContext to manage error state.
#[cfg(all(feature = "rt", feature = "syn"))]
#[inline]
pub fn attempt<T, F>(input: ParseStream, ctx: &mut ParseContext, parser: F) -> Result<Option<T>>
where
    F: FnOnce(ParseStream, &mut ParseContext) -> Result<T>,
{
    // Regular attempt without label
    attempt_labeled(input, ctx, None, parser)
}

#[cfg(all(feature = "rt", feature = "syn"))]
#[inline]
pub fn attempt_labeled<T, F>(
    input: ParseStream,
    ctx: &mut ParseContext,
    label: Option<&str>,
    parser: F,
) -> Result<Option<T>>
where
    F: FnOnce(ParseStream, &mut ParseContext) -> Result<T>,
{
    let was_fatal = ctx.check_fatal();
    ctx.set_fatal(false);

    // Snapshot symbol table, rule stack, and last_span
    let scopes_snapshot = ctx.scopes.clone();
    let rule_stack_snapshot = ctx.rule_stack.clone();
    let last_span_snapshot = ctx.last_span;

    let start_span = input.span();
    let fork = input.fork();

    // Pass ctx into the closure
    let res = parser(&fork, ctx);

    let is_now_fatal = ctx.check_fatal();

    match res {
        Ok(val) => {
            input.advance_to(&fork);
            ctx.set_fatal(was_fatal);
            // We KEEP the last_span updated by the successful attempt
            Ok(Some(val))
        }
        Err(e) => {
            if is_now_fatal {
                // Restore state
                ctx.scopes = scopes_snapshot;
                ctx.rule_stack = rule_stack_snapshot;
                ctx.last_span = last_span_snapshot;

                ctx.set_fatal(true);
                Err(e)
            } else {
                ctx.set_fatal(was_fatal);

                let suppress = ctx.suppress_label;
                ctx.suppress_label = false; // Reset

                // Determine label and priority logic
                // Rule: If error is at the start (no progress), we use the label and priority 1.
                // If error is deep (progress), we ignore label (pass None) and priority 0.

                let is_at_start = e.span().start() == start_span.start();

                let (final_label, priority) = if is_at_start && !suppress {
                    (
                        label.map(|s| s.to_string()),
                        if label.is_some() { 1 } else { 0 },
                    )
                } else {
                    (None, 0)
                };

                // Record error BEFORE restoring state to capture inner rule context
                // Note: We use the existing rule_stack (which might be deep) if we haven't popped yet.
                // But attempt() caller usually hasn't popped.
                // Wait, attempt() restores stack AFTER parser() returns.
                // So ctx.rule_stack is still the stack *inside* the attempt.
                // Actually, parser() should have exited its rules.

                // If the parser popped its rules, ctx.rule_stack is back to what it was when attempt started.
                // So we are recording with the outer stack!

                ctx.record_error(e, start_span, final_label, priority);

                // Restore state
                ctx.scopes = scopes_snapshot;
                ctx.rule_stack = rule_stack_snapshot;
                ctx.last_span = last_span_snapshot;

                Ok(None)
            }
        }
    }
}

/// Executes a parser on a fork, returning the result but NEVER advancing the input.
/// Restores ParseContext state (scopes, last_span) to what it was before.
#[cfg(all(feature = "rt", feature = "syn"))]
#[inline]
pub fn peek<T, F>(input: ParseStream, ctx: &mut ParseContext, parser: F) -> Result<T>
where
    F: FnOnce(ParseStream, &mut ParseContext) -> Result<T>,
{
    let fork = input.fork();

    // Snapshot state
    let scopes_snapshot = ctx.scopes.clone();
    let rule_stack_snapshot = ctx.rule_stack.clone();
    let last_span_snapshot = ctx.last_span;

    let res = parser(&fork, ctx);

    // Always restore state because we are peeking (state side effects should not persist)
    ctx.scopes = scopes_snapshot;
    ctx.rule_stack = rule_stack_snapshot;
    ctx.last_span = last_span_snapshot;

    res
}

/// Executes a parser on a fork.
/// If it SUCCEEDS, returns Err("unexpected match").
/// If it FAILS, returns Ok(()).
/// Never advances input. Restores state.
#[cfg(all(feature = "rt", feature = "syn"))]
#[inline]
pub fn not_check<T, F>(input: ParseStream, ctx: &mut ParseContext, parser: F) -> Result<()>
where
    F: FnOnce(ParseStream, &mut ParseContext) -> Result<T>,
{
    let fork = input.fork();

    // Snapshot state
    let scopes_snapshot = ctx.scopes.clone();
    let rule_stack_snapshot = ctx.rule_stack.clone();
    let last_span_snapshot = ctx.last_span;

    // Disable fatal errors for the check to allow backtracking/failure
    let was_fatal = ctx.check_fatal();
    ctx.set_fatal(false);

    let res = parser(&fork, ctx);

    // Restore fatal flag
    ctx.set_fatal(was_fatal);

    // Restore state
    ctx.scopes = scopes_snapshot;
    ctx.rule_stack = rule_stack_snapshot;
    ctx.last_span = last_span_snapshot;

    match res {
        Ok(_) => Err(syn::Error::new(input.span(), "unexpected match")),
        Err(_) => Ok(()),
    }
}

/// Wrapper around attempt used specifically for recovery blocks.
#[cfg(all(feature = "rt", feature = "syn"))]
#[inline]
pub fn attempt_recover<T, F>(
    input: ParseStream,
    ctx: &mut ParseContext,
    parser: F,
) -> Result<Option<T>>
where
    F: FnOnce(ParseStream, &mut ParseContext) -> Result<T>,
{
    let was_fatal = ctx.check_fatal();
    ctx.set_fatal(false);

    // Snapshot symbol table and rule stack
    let scopes_snapshot = ctx.scopes.clone();
    let rule_stack_snapshot = ctx.rule_stack.clone();
    let last_span_snapshot = ctx.last_span;

    let start_span = input.span();
    let fork = input.fork();

    let res = parser(&fork, ctx);

    // Always restore fatal state, ignoring whatever happened inside.
    ctx.set_fatal(was_fatal);

    match res {
        Ok(val) => {
            input.advance_to(&fork);
            // Keep last_span
            Ok(Some(val))
        }
        Err(e) => {
            // Record error BEFORE restoring state
            // Recovery attempts don't have labels (usually), so defaults are fine.
            ctx.record_error(e, start_span, None, 0);

            // Restore state
            ctx.scopes = scopes_snapshot;
            ctx.rule_stack = rule_stack_snapshot;
            ctx.last_span = last_span_snapshot;

            Ok(None)
        }
    }
}

// --- Combinators (High-Level Parsers) ---

#[cfg(all(feature = "rt", feature = "syn"))]
pub fn parse_separated<T, P, S>(
    input: ParseStream,
    ctx: &mut ParseContext,
    mut item_parser: P,
    mut sep_parser: S,
    min: usize,
    trailing: bool,
    error_msg: Option<&str>,
) -> Result<Vec<T>>
where
    P: FnMut(ParseStream, &mut ParseContext) -> Result<T>,
    S: FnMut(ParseStream, &mut ParseContext) -> Result<()>,
{
    let mut items = Vec::new();
    let mut first = true;

    loop {
        if !first {
            // Try parse separator
            let is_sep = attempt(input, ctx, |i, c| sep_parser(i, c))?.is_some();
            if !is_sep {
                break;
            }
        }

        // Try parse item
        // We use attempt because item parsing might fail deeply
        if let Some(item) = attempt(input, ctx, |i, c| item_parser(i, c))? {
            items.push(item);
            first = false;
        } else {
            // If we are here, we either:
            // 1. Just started (first=true) and failed to parse first item -> Empty list?
            // 2. Had a separator (first=false) but failed to parse item -> Trailing? or Error?

            if !first {
                if !trailing {
                    // We had a separator but no item, and trailing is NOT allowed.
                    let msg = error_msg.unwrap_or("expected item after separator");
                    return ctx.raise_failure(msg, input.span());
                }
                // Trailing allowed, so it's okay.
                break;
            } else {
                // First item failed. List is empty.
                break;
            }
        }
    }

    if items.len() < min {
        return ctx.raise_failure(format!("expected at least {} items", min), input.span());
    }

    Ok(items)
}

#[cfg(all(feature = "rt", feature = "syn"))]
pub fn parse_repeated<T, P>(
    input: ParseStream,
    ctx: &mut ParseContext,
    mut item_parser: P,
    min: usize,
) -> Result<Vec<T>>
where
    P: FnMut(ParseStream, &mut ParseContext) -> Result<T>,
{
    let mut items = Vec::new();

    while let Some(item) = attempt(input, ctx, |i, c| item_parser(i, c))? {
        items.push(item);
    }

    if items.len() < min {
        return ctx.raise_failure(format!("expected at least {} items", min), input.span());
    }

    Ok(items)
}

// --- Stateless Helpers (No Context Needed) ---

#[cfg(all(feature = "rt", feature = "syn"))]
#[inline]
pub fn parse_ident(input: ParseStream) -> Result<syn::Ident> {
    input.parse()
}

#[cfg(all(feature = "rt", feature = "syn"))]
#[inline]
pub fn parse_int<T: std::str::FromStr>(input: ParseStream) -> Result<T>
where
    T::Err: std::fmt::Display,
{
    input.parse::<syn::LitInt>()?.base10_parse()
}

#[cfg(all(feature = "rt", feature = "syn"))]
pub fn skip_until(input: ParseStream, predicate: impl Fn(ParseStream) -> bool) -> Result<()> {
    while !input.is_empty() && !predicate(input) {
        if input.parse::<proc_macro2::TokenTree>().is_err() {
            break;
        }
    }
    Ok(())
}

#[cfg(all(test, feature = "rt", feature = "syn"))]
mod tests {
    use super::*;

    #[test]
    fn test_rule_name_in_error() {
        let mut ctx = ParseContext::new();
        ctx.enter_rule("test_rule");

        let err = syn::Error::new(Span::call_site(), "expected something");
        ctx.record_error(err, Span::call_site(), None, 0);

        let final_err = ctx.take_best_error().unwrap();
        assert_eq!(
            final_err.to_string(),
            "in rule `test_rule`: expected something"
        );
    }

    #[test]
    fn test_nested_rule_name_in_error() {
        let mut ctx = ParseContext::new();
        ctx.enter_rule("outer");
        ctx.enter_rule("inner");

        let err = syn::Error::new(Span::call_site(), "fail");
        ctx.record_error(err, Span::call_site(), None, 0);

        let final_err = ctx.take_best_error().unwrap();
        assert_eq!(
            final_err.to_string(),
            "in rule `outer`: in rule `inner`: fail"
        );

        // Simulate outer rule recording it too
        ctx.exit_rule(); // inner popped

        // record the ALREADY FORMATTED error
        ctx.record_error(final_err, Span::call_site(), None, 0);

        let final_err2 = ctx.take_best_error().unwrap();

        // With prefix checking, it should stay the same
        assert_eq!(
            final_err2.to_string(),
            "in rule `outer`: in rule `inner`: fail"
        );
    }

    #[test]
    fn test_attempt_captures_rule_context() {
        use syn::parse::Parser;

        let mut ctx = ParseContext::new();

        let parser = |input: ParseStream| {
            ctx.enter_rule("outer");

            let _: Option<()> = attempt(input, &mut ctx, |_input, _ctx| {
                Err(syn::Error::new(Span::call_site(), "parse failed"))
            })?;

            ctx.exit_rule();
            Ok(())
        };

        let _ = parser.parse_str("");

        let err = ctx.take_best_error().expect("Error should be recorded");
        assert_eq!(err.to_string(), "in rule `outer`: parse failed");
    }

    #[test]
    fn test_raise_failure() {
        let mut ctx = ParseContext::new();
        let span = Span::call_site();

        // Record a normal error first
        ctx.record_error(syn::Error::new(span, "normal error"), span, None, 0);

        // Now raise a failure
        let res: Result<()> = ctx.raise_failure("critical failure", span);

        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "critical failure");

        // Assert that it is NOT fatal by default (reverted behavior)
        assert!(!ctx.check_fatal());

        // Best error should be cleared (or nullified) so raise_failure return value is used.
        // Actually raise_failure returns Err directly.
        // But if we record it?

        // The pattern for `fail` is: return Err from parser immediately.
        // The attempt() wrapper catches it.
        // If it's caught, record_error is called.
        // record_error sees `fail_triggered` is true, so priority becomes 2.
    }
}
