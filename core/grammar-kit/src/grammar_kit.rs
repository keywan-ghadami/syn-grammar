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

#[cfg(feature = "syn")]
pub mod syn_rt;

pub mod macros;

pub use grammar_kit_macros::with_span;

pub trait WithSpan<ParsedData> {
    /// Constructs the node from parsed data and a source span.
    fn with_span(parsed_data: ParsedData, span: std::ops::Range<usize>) -> Self;
}

/// Generic symbol table that tracks variable definitions in nested scopes.
/// This is essential for parsing languages with lexical scoping rules.
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

/// Represents the state of a single parse error, including its location,
/// priority, and the context in which it occurred. This is used by `ParseContext`
/// to track the "best" error encountered so far.
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

/// Holds the shared state for a parsing process, enabling advanced error
/// reporting, backtracking, and context-aware parsing. It is passed mutably
/// through the parser functions.
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
    pub pending_priority: u8,
    suppress_label: bool,
    mode_stack: Vec<bool>, // true = lexical, false = spaced
    group_depth: usize,
}

#[cfg(feature = "rt")]
impl ParseContext {
    // --- PRIORITY SYSTEM ---
    pub const PRIO_NORMAL: u8 = 0;
    pub const PRIO_LABELED: u8 = 10;
    pub const PRIO_AGGREGATED: u8 = 20;
    pub const PRIO_STRUCTURAL: u8 = 50;

    pub fn set_priority(&mut self, prio: u8) {
        if prio > self.pending_priority {
            self.pending_priority = prio;
        }
    }

    pub fn new() -> Self {
        Self {
            is_fatal: false,
            #[cfg(feature = "syn")]
            best_error: None,
            scopes: ScopeStack::new(),
            rule_stack: Vec::new(),
            #[cfg(feature = "syn")]
            last_span: None,
            pending_priority: Self::PRIO_NORMAL,
            suppress_label: false,
            mode_stack: Vec::new(),
            group_depth: 0,
        }
    }

    pub fn enter_group(&mut self) { self.group_depth += 1; }
    pub fn exit_group(&mut self) { self.group_depth = self.group_depth.saturating_sub(1); }
    pub fn is_in_group(&self) -> bool { self.group_depth > 0 }

    pub fn set_fatal(&mut self, fatal: bool) {
        self.is_fatal = fatal;
    }

    pub fn check_fatal(&self) -> bool {
        self.is_fatal
    }

    pub fn trigger_fail(&mut self) {
        self.set_priority(Self::PRIO_STRUCTURAL);
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
        self.set_priority(Self::PRIO_STRUCTURAL);
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

    pub fn current_rule_name(&self) -> Option<&str> {
        self.rule_stack.last().map(|s| s.as_str())
    }

    /// Records an error if it is "better" than the current best error.
    /// The "best" error is determined by a set of heuristics, including whether
    /// the error is fatal, how far it is in the input, and its priority.
    #[cfg(feature = "syn")]
    pub fn record_error(
        &mut self,
        err: syn::Error,
        _attempt_span: Span,
        label: Option<String>,
        mut priority: u8,
    ) {
        let err_str = err.to_string();
        // 1. FILTER DUMMY BUBBLE ERROR
        if err_str.contains("__DUMMY_ERR_BUBBLE__") || err_str.contains("__BUBBLE__") {
            #[cfg(feature = "trace")]
            eprintln!("[TRACE] record_error: Ignoring bubble error");
            return;
        }

        // Trace every considered error before any filtering.
        #[cfg(feature = "trace")]
        eprintln!("[TRACE] considering_error: '{}' (priority: {}, label: {:?})", err_str, priority, label);
        
        // Eskalierte Priorität aus dem Kontext übernehmen und sofort zurücksetzen
        priority = std::cmp::max(priority, self.pending_priority);
        self.pending_priority = Self::PRIO_NORMAL;

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
                #[cfg(feature = "trace")]
                eprintln!("[TRACE] record_error: New best error (no previous)");
                self.best_error = Some(new_error_state);
            }
            Some(existing) => {
                // 1. Fatality: If the new error is fatal, it wins immediately.
                if new_error_state.is_fatal && !existing.is_fatal {
                    #[cfg(feature = "trace")]
                    eprintln!("[TRACE] record_error: New best error (is fatal)");
                    self.best_error = Some(new_error_state);
                    return;
                }
                if existing.is_fatal && !new_error_state.is_fatal {
                    #[cfg(feature = "trace")]
                    eprintln!("[TRACE] record_error: Rejected (existing is fatal)");
                    return;
                }

                // 2. Location (Progress) -> DIES MUSS GEWINNEN, EGAL WELCHE PRIORITÄT!
                let new_start = new_error_state.start_span.start();
                let old_start = existing.start_span.start();

                let is_deeper = new_start.line > old_start.line
                    || (new_start.line == old_start.line && new_start.column > old_start.column);

                let is_shallower = old_start.line > new_start.line
                    || (old_start.line == new_start.line && old_start.column > new_start.column);

                if is_deeper {
                    #[cfg(feature = "trace")]
                    eprintln!("[TRACE] record_error: New best error (deeper span)");
                    self.best_error = Some(new_error_state);
                    return;
                } else if is_shallower {
                    #[cfg(feature = "trace")]
                    eprintln!("[TRACE] record_error: Rejected (shallower span)");
                    return;
                }

                // 3. Priority (Nur wenn an exakt der gleichen Stelle!)
                if new_error_state.priority > existing.priority {
                    #[cfg(feature = "trace")]
                    eprintln!("[TRACE] record_error: New best error (higher priority)");
                    self.best_error = Some(new_error_state);
                    return;
                } else if existing.priority > new_error_state.priority {
                    #[cfg(feature = "trace")]
                    eprintln!("[TRACE] record_error: Rejected (lower priority)");
                    return;
                }

                // 4. Context specificity
                if new_error_state.rule_stack.len() > existing.rule_stack.len()
                    || (new_error_state.label.is_some() && existing.label.is_none())
                {
                    #[cfg(feature = "trace")]
                    eprintln!("[TRACE] record_error: New best error (more specific context)");
                    self.best_error = Some(new_error_state);
                } else if new_error_state.rule_stack.len() == existing.rule_stack.len() {
                    // Tie-breaker: Check if one message is a generic fallback
                    let is_new_generic = new_error_state.err.to_string().contains("No matching");
                    let is_existing_generic = existing.err.to_string().contains("No matching");

                    if is_existing_generic && !is_new_generic {
                        #[cfg(feature = "trace")]
                        eprintln!("[TRACE] record_error: New best error (replaces generic fallback)");
                        self.best_error = Some(new_error_state);
                        return;
                    } else if is_new_generic && !is_existing_generic {
                        #[cfg(feature = "trace")]
                        eprintln!("[TRACE] record_error: Rejected (new is generic fallback)");
                        return;
                    }

                    // TIE-BREAKER: Längere Message gewinnt (Sicherster Fallback)
                    if new_error_state.err.to_string().len() >= existing.err.to_string().len() {
                        self.best_error = Some(new_error_state);
                    }
                } else {
                    #[cfg(feature = "trace")]
                    eprintln!("[TRACE] record_error: Rejected (less specific context)");
                }
            }
        }
    }

    #[cfg(feature = "syn")]
    pub fn take_best_error(&mut self) -> Option<syn::Error> {
        let best = self.best_error.take()?;
        let mut msg;

        let original_err_str = best.err.to_string();

        if best.priority >= Self::PRIO_LABELED && best.priority < Self::PRIO_STRUCTURAL && best.label.is_some() {
            let label = best.label.as_ref().unwrap();
            // Prepend "unexpected end of input" for clarity if that was the root cause.
            if original_err_str.contains("unexpected end of input") {
                msg = format!("unexpected end of input, expected {}", label);
            } else {
                if !original_err_str.contains("expected") {
                    msg = format!("expected {}", label);
                } else {
                    msg = format!("expected {}", label);
                }
            }
        } else {
            msg = original_err_str;
        }

        let line = best.start_span.start().line;
        let col = best.start_span.start().column;
        
        // ORIGINAL POSITION: Put column info directly after the main message
        if !msg.contains(&format!("at column {}", col)) {
            msg = format!("{} at column {} (line {})", msg, col, line);
        }

        // Add the rule stack for context, avoiding duplicates.
        if !best.rule_stack.is_empty() {
            for rule in best.rule_stack.iter().rev() {
                let suffix = format!("\nin {}", rule);
                if !msg.contains(&suffix) {
                    msg = format!("{}{}", msg, suffix);
                }
            }
        }

        Some(syn::Error::new(best.start_span, msg))
    }

    /// Determines if the current best error is "significant enough" to stop
    /// parsing alternatives. A significant error is one that is fatal,
    /// has a high priority, or occurred after making some progress.
    #[cfg(feature = "syn")]
    pub fn stop_aggregation(&self, current_span: Span) -> bool {
        if let Some(e) = &self.best_error {
            // 1. Fatal errors always stop everything.
            if e.is_fatal {
                return true;
            }

            // 2. High-priority errors (e.g., from `fail!`) are significant.
            // WICHTIGE ÄNDERUNG: Für Listen darf ein "Labeled" Fehler (Prio 10) den Abbruch *nicht* triggern,
            // sonst kann eine leere/abbrechende Liste nicht sauber beendet werden, sondern wirft sofort Err.
            // Nur Structural-Fehler dürfen das.
            if e.priority >= Self::PRIO_STRUCTURAL {
                return true;
            }

            // 3. Deep errors, where the error occurred after the current starting point, are significant.
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

    // --- Span Tracking & Lexical Mode ---

    pub fn enter_lexical(&mut self) {
        self.mode_stack.push(true);
    }

    pub fn enter_spaced(&mut self) {
        self.mode_stack.push(false);
    }

    pub fn exit_mode(&mut self) {
        self.mode_stack.pop();
    }

    pub fn is_lexical(&self) -> bool {
        *self.mode_stack.last().unwrap_or(&false)
    }

    #[cfg(feature = "syn")]
    pub fn record_span(&mut self, span: Span) -> Result<()> {
        if self.is_lexical() {
            if let Some(last) = self.last_span {
                if last.end() != span.start() {
                    return Err(syn::Error::new(span, "expected no whitespace"));
                }
            }
        }
        self.last_span = Some(span);
        Ok(())
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
/// If the inner parser succeeds, it advances the input stream.
/// If it fails, it backtracks, restores the parsing context, and records the error.
#[cfg(all(feature = "rt", feature = "syn"))]
#[inline]
pub fn attempt<T, F>(input: ParseStream, ctx: &mut ParseContext, parser: F) -> Result<Option<T>>
where
    F: FnOnce(ParseStream, &mut ParseContext) -> Result<T>,
{
    // Regular attempt without label
    attempt_labeled(input, ctx, None, parser)
}

/// A variant of `attempt` that attaches a descriptive label to errors that
/// occur at the beginning of the parse attempt. This improves error messages for alternatives.
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

    // Snapshot state
    let scopes_snapshot = ctx.scopes.clone();
    let rule_stack_snapshot = ctx.rule_stack.clone();
    let last_span_snapshot = ctx.last_span;
    let mode_stack_snapshot = ctx.mode_stack.clone();
    let best_error_snapshot = ctx.best_error.clone();

    let start_span = input.span();
    let fork = input.fork();
    let res = parser(&fork, ctx);
    let is_now_fatal = ctx.check_fatal();

    match res {
        Ok(val) => {
            input.advance_to(&fork);
            ctx.set_fatal(was_fatal);
            ctx.mode_stack = mode_stack_snapshot;

            // THE HIGH-WATER MARK FIX:
            // Wir behalten den inneren Fehler NUR, wenn er tiefer im Input 
            // aufgetreten ist, als dieser Versuch gestartet wurde (echter Fortschritt).
            // Zero-Progress-Fehler aus erfolgreichen aber leeren Zweigen werden verworfen.
            let keep_error = match &ctx.best_error {
                Some(e) => {
                    let e_start = e.start_span.start();
                    let s_start = start_span.start();
                    e_start.line > s_start.line || (e_start.line == s_start.line && e_start.column > s_start.column)
                }
                None => false,
            };

            if !keep_error {
                ctx.best_error = best_error_snapshot;
            }

            Ok(Some(val))
        }
        Err(e) => {
            // BUBBLE CHECK: If the error is a bubble, we abort immediately and pass it upwards!
            if e.to_string().contains("__BUBBLE__") || e.to_string().contains("__DUMMY_ERR_BUBBLE__") {
                ctx.scopes = scopes_snapshot;
                ctx.rule_stack = rule_stack_snapshot;
                ctx.last_span = last_span_snapshot;
                ctx.mode_stack = mode_stack_snapshot;
                // Fatal states bubble too
                ctx.set_fatal(was_fatal || is_now_fatal);
                return Err(e);
            }

            let is_at_start = e.span().start() == start_span.start();

            // On any failure, restore the non-error-related context.
            ctx.scopes = scopes_snapshot;
            ctx.rule_stack = rule_stack_snapshot;
            ctx.last_span = last_span_snapshot;
            ctx.mode_stack = mode_stack_snapshot;

            if is_now_fatal {
                // Fatal error. This branch is now definitive.
                ctx.set_fatal(true);
                return Err(e);
            }

            // Recoverable failure. Restore original fatal flag.
            ctx.set_fatal(was_fatal);
            
            let suppress = ctx.suppress_label;
            ctx.suppress_label = false;

            // KORREKTE HIGH-WATER MARK:
            // Wenn der aktuelle Parse-Versuch KEINEN Fortschritt gemacht hat (Fehler direkt am Start),
            // dann setzen wir den Error-State auf den Snapshot ZURÜCK.
            // WARUM DAS FUNKTIONIERT: Wenn ein vorheriger OR-Zweig bereits einen tiefen Fehler
            // geworfen hatte, ist dieser tiefe Fehler IM SNAPSHOT gespeichert und wird hiermit restauriert!
            if is_at_start {
                ctx.best_error = best_error_snapshot;
            }

            if !suppress {
                if let Some(lbl) = label {
                    if is_at_start {
                        ctx.record_error(e, start_span, Some(lbl.to_string()), ParseContext::PRIO_LABELED);
                    } else {
                        ctx.record_error(e, start_span, None, ParseContext::PRIO_NORMAL);
                    }
                } else {
                    ctx.record_error(e, start_span, None, ParseContext::PRIO_NORMAL);
                }
            }

            Ok(None)
        }
    }
}

/// Executes a parser on a fork, returning the result but NEVER advancing the input stream.
/// This is useful for looking ahead in the token stream. State is always restored.
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
    let mode_stack_snapshot = ctx.mode_stack.clone();
    let best_error_snapshot = ctx.best_error.clone();

    let res = parser(&fork, ctx);
    
    // Always restore state because we are peeking (state side effects should not persist)
    ctx.scopes = scopes_snapshot;
    ctx.rule_stack = rule_stack_snapshot;
    ctx.last_span = last_span_snapshot;
    ctx.mode_stack = mode_stack_snapshot;
    ctx.best_error = best_error_snapshot;

    res
}

/// Executes a parser on a fork.
/// If it SUCCEEDS, returns an `Err`.
/// If it FAILS, returns `Ok(())`.
/// This is used for negative lookaheads. The input stream is never advanced.
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
    let mode_stack_snapshot = ctx.mode_stack.clone();
    let best_error_snapshot = ctx.best_error.clone();

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
    ctx.mode_stack = mode_stack_snapshot;
    ctx.best_error = best_error_snapshot;

    match res {
        Ok(_) => Err(syn::Error::new(input.span(), "unexpected match")),
        Err(_) => Ok(()),
    }
}

/// Wrapper around attempt used specifically for recovery blocks.
/// This allows parsing to continue even after an error, which is useful
/// for providing more comprehensive diagnostics.
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
    let mode_stack_snapshot = ctx.mode_stack.clone();
    let best_error_snapshot = ctx.best_error.clone();

    let start_span = input.span();
    let fork = input.fork();
    let res = parser(&fork, ctx);

    // Always restore fatal state, ignoring whatever happened inside.
    ctx.set_fatal(was_fatal);
    
    match res {
        Ok(val) => {
            input.advance_to(&fork);
            // Keep last_span
            // Restore mode stack
            ctx.mode_stack = mode_stack_snapshot;

            // HIGH-WATER MARK FIX:
            let keep_error = match &ctx.best_error {
                Some(e) => {
                    let e_start = e.start_span.start();
                    let s_start = start_span.start();
                    e_start.line > s_start.line || (e_start.line == s_start.line && e_start.column > s_start.column)
                }
                None => false,
            };

            if !keep_error {
                ctx.best_error = best_error_snapshot;
            }

            Ok(Some(val))
        }
        Err(e) => {
            if e.to_string().contains("__BUBBLE__") || e.to_string().contains("__DUMMY_ERR_BUBBLE__") {
                return Err(e);
            }

            // Record error BEFORE restoring state
            // Recovery attempts don't have labels (usually), so defaults are fine.
            ctx.record_error(e, start_span, None, 0);

            // Restore state
            ctx.scopes = scopes_snapshot;
            ctx.rule_stack = rule_stack_snapshot;
            ctx.last_span = last_span_snapshot;
            ctx.mode_stack = mode_stack_snapshot;

            Ok(None)
        }
    }
}

#[cfg(all(feature = "rt", feature = "syn"))]
pub fn parse_separated<T, P, S>(
    input: ParseStream,
    ctx: &mut ParseContext,
    mut item_parser: P,
    mut sep_parser: S,
    min: usize,
    trailing: bool,
    item_name: &str,
) -> Result<Vec<T>>
where
    P: FnMut(ParseStream, &mut ParseContext) -> Result<T>,
    S: FnMut(ParseStream, &mut ParseContext) -> Result<()> ,
{
    let mut items = Vec::new();
    
    // 1. Parse the first item.
    let first_item_span = input.span();
    ctx.enter_rule(&format!("{} 1", item_name));
    let first_item = match attempt_labeled(input, ctx, Some(item_name), |i, c| item_parser(i, c)) {
        Ok(Some(item)) => {
            ctx.exit_rule();
            item
        }
        Ok(None) => {
            ctx.exit_rule();
            if ctx.stop_aggregation(first_item_span) {
                return Err(syn::Error::new(first_item_span, "__BUBBLE__"));
            }
            // Not enough items will be checked at the end.
            return Ok(items);
        }
        Err(e) => {
            ctx.exit_rule();
            return Err(e); // Propagate fatal error or bubble
        }
    };
    items.push(first_item);

    // 2. Loop for subsequent items.
    loop {
        let pre_sep_span = input.span();
        let fork = input.fork();
        ctx.enter_rule("separator");
        let sep_res = attempt(&fork, ctx, |i, c| sep_parser(i, c));
        ctx.exit_rule();
        
        match sep_res {
            Ok(Some(_)) => {
                // Separator found, commit.
                input.advance_to(&fork);
            }
            Ok(None) => {
                // No separator, end of list.
                if ctx.stop_aggregation(pre_sep_span) {
                    return Err(syn::Error::new(pre_sep_span, "__BUBBLE__"));
                }
                break;
            }
            Err(e) => return Err(e), // Fatal error or bubble
        }

        if trailing && input.is_empty() {
            break;
        }

        let next_idx = items.len() + 1;
        let rule_name = format!("{} {}", item_name, next_idx);
        ctx.enter_rule(&rule_name);
        let item_res = attempt_labeled(input, ctx, Some(item_name), |i, c| item_parser(i, c));
        ctx.exit_rule();
        
        match item_res {
            Ok(Some(item)) => items.push(item),
            Ok(None) => {
                // After a separator, an item is mandatory.
                // We don't take_best_error, we just trigger a bubble so the real error is preserved.
                // If there somehow isn't a best error, provide a fallback.
                if ctx.best_error.is_none() {
                    let msg = format!("invalid item after separator for {}", item_name);
                    ctx.record_error(syn::Error::new(input.span(), &msg), input.span(), None, ParseContext::PRIO_NORMAL);
                }
                return Err(syn::Error::new(input.span(), "__BUBBLE__"));
            }
            Err(e) => return Err(e), // Fatal error or bubble
        }
    }

    // 3. Final check for minimum number of items.
    if items.len() < min {
        return ctx.raise_failure(
            &format!(
                "expected at least {} {}s, found {}",
                min, item_name, items.len()
            ),
            input.span(),
        );
    }

    Ok(items)
}

/// A combinator for parsing a repetition of an item.
/// It continues parsing until the item parser fails and handles a minimum number of items.
#[cfg(all(feature = "rt", feature = "syn"))]
pub fn parse_repeated<T, P>(
    input: ParseStream,
    ctx: &mut ParseContext,
    mut item_parser: P,
    min: usize,
    item_name: &str, // CRITICAL: Kontext-Name
) -> Result<Vec<T>>
where
    P: FnMut(ParseStream, &mut ParseContext) -> Result<T>,
{
    let mut items = Vec::new();
    loop {
        let loop_start_span = input.span();
        let next_idx = items.len() + 1;
        let rule_name = format!("{} {}", item_name, next_idx);

        ctx.enter_rule(&rule_name);
        let item = match attempt_labeled(input, ctx, Some(item_name), |i, c| item_parser(i, c)) {
            Ok(Some(item)) => item,
            Ok(None) => {
                ctx.exit_rule();
                if ctx.stop_aggregation(loop_start_span) {
                    return Err(syn::Error::new(loop_start_span, "__BUBBLE__"));
                }
                break; // End of repetition
            }
            Err(e) => {
                ctx.exit_rule();
                return Err(e); // Propagate fatal error or bubble
            }
        };
        ctx.exit_rule();
        items.push(item);
    }

    if items.len() < min {
        return ctx.raise_failure(
            &format!("expected at least {} {}s, found {}", min, item_name, items.len()),
            input.span()
        );
    }

    Ok(items)
}

// --- Delimited Parsing ---

/// A helper for parsing content enclosed in delimiters like `()`, `{}`, or `[]`.
/// It ensures that the entire content within the delimiters is consumed by the inner parser.
#[cfg(all(feature = "rt", feature = "syn"))]
pub fn parse_delimited<T, F>(
    input: ParseStream,
    ctx: &mut ParseContext,
    parser: F,
    delimiter: char,
) -> Result<T>
where
    F: FnOnce(ParseStream, &mut ParseContext) -> Result<T>,
{
    let content;
    let final_span: proc_macro2::Span;
    match delimiter {
        '(' => {
            let paren_token = syn::parenthesized!(content in input);
            final_span = paren_token.span.join();
        }
        '{' => {
            let brace_token = syn::braced!(content in input);
            final_span = brace_token.span.join();
        }
        '[' => {
            let bracket_token = syn::bracketed!(content in input);
            final_span = bracket_token.span.join();
        }
        _ => {
            return Err(syn::Error::new(
                input.span(),
                "unsupported delimiter for custom parsing",
            ));
        }
    }
    ctx.record_span(final_span)?;

    // Run the inner parser on the content stream.
    ctx.enter_group();
    let res = parser(&content, ctx);
    ctx.exit_group();

    match res {
        Ok(val) => {
            // On success, check if all tokens were consumed.
            // If not, it's an "unexpected token" error.
            if !content.is_empty() {
                // Before declaring a structural error, check if the inner parser
                // recorded a significant error but still returned Ok (e.g. from a buggy combinator).
                if ctx.stop_aggregation(content.span()) {
                    return Err(syn::Error::new(content.span(), "__BUBBLE__"));
                }

                let err = content.error("unexpected token in delimited group");
                // WICHTIG: Setze dies auf PRIO_NORMAL, damit es spezifischere (tiefere) Item-Fehler
                // in der Liste nicht überschreibt!
                ctx.record_error(err, content.span(), None, ParseContext::PRIO_NORMAL);
                return Err(syn::Error::new(content.span(), "__BUBBLE__"));
            } else {
                Ok(val)
            }
        }
        Err(e) => {
            // If the inner parser returns a hard error (like bubble), we propagate it directly.
            Err(e)
        }
    }
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
        
        // Updated expectation to match new format
        assert!(
            final_err.to_string().contains("expected something") &&
            final_err.to_string().contains("in test_rule")
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
        let final_err_str = final_err.to_string();

        assert!(final_err_str.contains("fail"));
        assert!(final_err_str.contains("in inner"));
        assert!(final_err_str.contains("in outer"));

        // Simulate outer rule recording it too
        ctx.exit_rule();
        // inner popped

        // record the ALREADY FORMATTED error
        ctx.record_error(final_err, Span::call_site(), None, 0);
        
        let final_err2 = ctx.take_best_error().unwrap();
        let final_err2_str = final_err2.to_string();

        // With prefix checking, it should stay the same
        assert!(final_err2_str.contains("fail"));
        assert!(final_err2_str.contains("in inner"));
        assert!(final_err2_str.contains("in outer"));
    }

    #[test]
    fn test_attempt_captures_rule_context() {
        use syn::parse::Parser;
        
        let mut ctx = ParseContext::new();

        let parser = |input: ParseStream| {
            ctx.enter_rule("outer");
            
            let _ = match attempt(input, &mut ctx, |_input, _ctx| {
                Err::<(), syn::Error>(syn::Error::new(Span::call_site(), "parse failed"))
            }) {
                Ok(Some(_)) => Ok(()),
                Ok(None) => Ok(()),
                Err(e) => Err(e),
            }?;

            ctx.exit_rule();
            Ok(())
        };
        
        let _ = parser.parse_str("");

        let err = ctx.take_best_error().expect("Error should be recorded");
        assert!(err.to_string().contains("parse failed"));
        assert!(err.to_string().contains("in outer"));
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
        
        // The pattern for `fail` is: return Err from parser immediately.
        // The attempt() wrapper catches it.
        // If it's caught, record_error is called.
        // record_error sees `fail_triggered` is true, so priority becomes 2.
    }
}
