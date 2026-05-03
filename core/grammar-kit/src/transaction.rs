// transaction.rs

pub struct ParseTransaction<'a> {
    ctx: &'a mut ParseContext,
    // Snapshots
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
            best_error_snapshot: ctx.error_tracker.best_error.clone(), // Gekapselt
            was_fatal,
            start_span,
            ctx,
        }
    }

    /// Wird aufgerufen, wenn der Spekulations-Zweig erfolgreich war.
    pub fn commit(self) {
        self.ctx.set_fatal(self.was_fatal);
        // Fehler-Bereinigung bei Zero-Progress gekapselt in error_tracker
        self.ctx.error_tracker.clean_zero_progress_errors(self.start_span, &self.best_error_snapshot);
    }

    /// Wird aufgerufen, wenn der Spekulations-Zweig fehlschlägt.
    /// Restauriert automatisch und garantiert den Zustand.
    pub fn rollback(self, error: syn::Error, label: Option<&str>) -> syn::Error {
        let is_now_fatal = self.ctx.check_fatal();
        
        // 1. Sichere Wiederherstellung des sauberen Zustands
        self.ctx.scopes = self.scopes_snapshot;
        self.ctx.rule_stack = self.rule_stack_snapshot;
        self.ctx.last_span = self.last_span_snapshot;
        self.ctx.mode_stack = self.mode_stack_snapshot;
        
        // 2. Fatal-Handling
        if is_now_fatal || error.to_string().contains("__BUBBLE__") {
            self.ctx.set_fatal(true);
            return error;
        }
        
        self.ctx.set_fatal(self.was_fatal);

        // 3. Fehler verarbeiten
        self.ctx.error_tracker.handle_rollback(
            error.clone(), 
            self.start_span, 
            label, 
            self.best_error_snapshot
        );

        error
    }
}
