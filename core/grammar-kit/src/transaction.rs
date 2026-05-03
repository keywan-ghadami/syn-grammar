use proc_macro2::Span;
use syn::Error;
use crate::{ParseContext, ErrorState, ScopeStack};

/// Kapselt einen spekulativen Parse-Versuch. Sichert beim Start den Zustand 
/// und garantiert bei einem Rollback die saubere Wiederherstellung.
pub struct ParseTransaction<'a> {
    ctx: &'a mut ParseContext,
    scopes_snapshot: ScopeStack,
    rule_stack_snapshot: Vec<String>,
    last_span_snapshot: Option<Span>,
    mode_stack_snapshot: Vec<bool>,
    best_error_snapshot: Option<ErrorState>,
    was_fatal: bool,
    start_span: Span,
}

impl<'a> ParseTransaction<'a> {
    pub fn begin(ctx: &'a mut ParseContext, start_span: Span) -> Self {
        let was_fatal = ctx.check_fatal();
        ctx.set_fatal(false);

        Self {
            scopes_snapshot: ctx.scopes.clone(),
            rule_stack_snapshot: ctx.rule_stack.clone(),
            last_span_snapshot: ctx.last_span,
            mode_stack_snapshot: ctx.mode_stack.clone(),
            best_error_snapshot: ctx.best_error.clone(),
            was_fatal,
            start_span,
            ctx,
        }
    }

    /// Wird bei einem erfolgreichen Parse-Versuch aufgerufen.
    /// Führt die High-Water Mark Prüfung durch, um Zero-Progress-Fehler 
    /// aus optionalen Zweigen zu bereinigen.
    pub fn commit(self) {
        self.ctx.set_fatal(self.was_fatal);

        let keep_error = match &self.ctx.best_error {
            Some(e) => {
                let e_start = e.start_span.start();
                let s_start = self.start_span.start();
                e.priority >= ParseContext::PRIO_AGGREGATED ||
                e_start.line > s_start.line || (e_start.line == s_start.line && e_start.column > s_start.column)
            }
            None => false,
        };

        if !keep_error {
            self.ctx.best_error = self.best_error_snapshot;
        }
    }

    /// Wird bei einem fehlgeschlagenen Parse-Versuch aufgerufen.
    /// Restauriert den Zustand und merget den neuen Fehler intelligent in den Kontext.
    pub fn rollback(self, error: Error, label: Option<&str>) -> Error {
        let is_now_fatal = self.ctx.check_fatal();
        
        let is_bubble = error.to_string().contains("__BUBBLE__") 
                     || error.to_string().contains("__DUMMY_ERR_BUBBLE__");

        // 1. Sichere Wiederherstellung des sauberen Zustands
        self.ctx.scopes = self.scopes_snapshot;
        self.ctx.rule_stack = self.rule_stack_snapshot;
        self.ctx.last_span = self.last_span_snapshot;
        self.ctx.mode_stack = self.mode_stack_snapshot;

        // 2. Fatal & Bubble Handling
        if is_bubble {
            self.ctx.set_fatal(self.was_fatal || is_now_fatal);
            return error;
        }

        if is_now_fatal {
            self.ctx.set_fatal(true);
            return error;
        }

        self.ctx.set_fatal(self.was_fatal);
        let suppress = self.ctx.suppress_label;
        self.ctx.suppress_label = false;

        // 3. Progress = Preservation (Fehlerbereinigung)
        let keep_error = match &self.ctx.best_error {
            Some(best) => {
                let b_start = best.start_span.start();
                let s_start = self.start_span.start();
                best.priority >= ParseContext::PRIO_AGGREGATED ||
                b_start.line > s_start.line || (b_start.line == s_start.line && b_start.column > s_start.column)
            }
            None => false,
        };

        if !keep_error {
            self.ctx.best_error = self.best_error_snapshot;
        }

        // 4. Fehleraufzeichnung
        if !suppress {
            let is_at_start = error.span().start() == self.start_span.start();
            if is_at_start {
                if let Some(lbl) = label {
                    self.ctx.record_error(error.clone(), self.start_span, Some(lbl.to_string()), ParseContext::PRIO_LABELED);
                } else {
                    self.ctx.record_error(error.clone(), self.start_span, None, ParseContext::PRIO_NORMAL);
                }
            } else {
                self.ctx.record_error(error.clone(), self.start_span, None, ParseContext::PRIO_NORMAL);
            }
        }

        error
    }

    /// Spezielle Rollback-Logik für Recovery-Blöcke
    pub fn rollback_for_recovery(self, error: Error) -> Error {
        if error.to_string().contains("__BUBBLE__") || error.to_string().contains("__DUMMY_ERR_BUBBLE__") {
            return error;
        }

        // Record error BEFORE restoring state
        self.ctx.record_error(error.clone(), self.start_span, None, 0);

        self.ctx.scopes = self.scopes_snapshot;
        self.ctx.rule_stack = self.rule_stack_snapshot;
        self.ctx.last_span = self.last_span_snapshot;
        self.ctx.mode_stack = self.mode_stack_snapshot;

        let keep_error = match &self.ctx.best_error {
            Some(best) => {
                let b_start = best.start_span.start();
                let s_start = self.start_span.start();
                best.priority >= ParseContext::PRIO_AGGREGATED ||
                b_start.line > s_start.line || (b_start.line == s_start.line && b_start.column > s_start.column)
            }
            None => false,
        };

        if !keep_error {
            self.ctx.best_error = self.best_error_snapshot;
        }

        error
    }
}
