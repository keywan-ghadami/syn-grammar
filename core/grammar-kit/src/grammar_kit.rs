#![doc = include_str!("../README.md")]

#[cfg(feature = "syn")]
use proc_macro2::Span;
use std::collections::HashSet;
#[cfg(feature = "syn")]
use syn::Result;

#[cfg(feature = "testing")]
pub mod testing;

#[cfg(feature = "syn")]
pub mod syn_rt;

pub mod macros;

// --- NEUE SUB-MODULE ---
#[cfg(all(feature = "rt", feature = "syn"))]
pub mod transaction;
#[cfg(all(feature = "rt", feature = "syn"))]
pub mod combinators;

pub use grammar_kit_macros::with_span;

// Exportiere alle Kombinatoren direkt, damit bestehender Code nicht bricht
#[cfg(all(feature = "rt", feature = "syn"))]
pub use combinators::*;

pub trait WithSpan<ParsedData> {
    fn with_span(parsed_data: ParsedData, span: std::ops::Range<usize>) -> Self;
}

#[derive(Clone, Default)]
pub struct ScopeStack {
    pub(crate) scopes: Vec<HashSet<String>>,
}

impl ScopeStack {
    pub fn new() -> Self { Self { scopes: vec![HashSet::new()] } }
    pub fn enter_scope(&mut self) { self.scopes.push(HashSet::new()); }
    pub fn exit_scope(&mut self) { if self.scopes.len() > 1 { self.scopes.pop(); } }
    pub fn define(&mut self, name: impl Into<String>) {
        if let Some(scope) = self.scopes.last_mut() { scope.insert(name.into()); }
    }
    pub fn is_defined(&self, name: &str) -> bool {
        for scope in self.scopes.iter().rev() { if scope.contains(name) { return true; } }
        false
    }
    pub fn scopes(&self) -> &Vec<HashSet<String>> { &self.scopes }
}

#[cfg(all(feature = "rt", feature = "syn"))]
#[derive(Clone, Debug)]
pub(crate) struct ErrorState {
    pub(crate) err: syn::Error,
    pub(crate) rule_stack: Vec<String>,
    pub(crate) start_span: Span,
    pub(crate) priority: u8,
    pub(crate) is_fatal: bool,
    pub(crate) label: Option<String>,
}

#[cfg(feature = "rt")]
#[derive(Clone)]
pub struct ParseContext {
    pub(crate) is_fatal: bool,
    #[cfg(feature = "syn")]
    pub(crate) best_error: Option<ErrorState>,
    pub scopes: ScopeStack,
    pub(crate) rule_stack: Vec<String>,
    #[cfg(feature = "syn")]
    pub last_span: Option<Span>,
    pub pending_priority: u8,
    pub(crate) suppress_label: bool,
    pub(crate) mode_stack: Vec<bool>, 
    pub(crate) group_depth: usize,
}

#[cfg(feature = "rt")]
impl ParseContext {
    pub const PRIO_NORMAL: u8 = 0;
    pub const PRIO_LABELED: u8 = 10;
    pub const PRIO_AGGREGATED: u8 = 20;
    pub const PRIO_STRUCTURAL: u8 = 50;

    pub fn set_priority(&mut self, prio: u8) {
        if prio > self.pending_priority { self.pending_priority = prio; }
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
    pub fn set_fatal(&mut self, fatal: bool) { self.is_fatal = fatal; }
    pub fn check_fatal(&self) -> bool { self.is_fatal }
    pub fn trigger_fail(&mut self) { self.set_priority(Self::PRIO_STRUCTURAL); }
    pub fn suppress_label(&mut self) { self.suppress_label = true; }
    pub fn commit(&mut self) { self.is_fatal = true; }

    #[cfg(feature = "syn")]
    pub fn raise_failure<T>(&mut self, msg: impl std::fmt::Display, span: Span) -> Result<T> {
        self.set_priority(Self::PRIO_STRUCTURAL);
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
        if let Some(n) = _name { eprintln!("[TRACE] exit_rule: {}", n); }
    }

    pub fn current_rule_name(&self) -> Option<&str> {
        self.rule_stack.last().map(|s| s.as_str())
    }

    #[cfg(feature = "syn")]
    pub fn record_error(&mut self, err: syn::Error, _attempt_span: Span, label: Option<String>, mut priority: u8) {
        let err_str = err.to_string();
        if err_str.contains("__DUMMY_ERR_BUBBLE__") || err_str.contains("__BUBBLE__") { return; }

        #[cfg(feature = "trace")]
        eprintln!("[TRACE] considering_error: '{}' (priority: {}, label: {:?})", err_str, priority, label);
        
        priority = std::cmp::max(priority, self.pending_priority);
        self.pending_priority = Self::PRIO_NORMAL;

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
                if new_error_state.is_fatal && !existing.is_fatal {
                    self.best_error = Some(new_error_state);
                    return;
                }
                if existing.is_fatal && !new_error_state.is_fatal { return; }

                let new_start = new_error_state.start_span.start();
                let old_start = existing.start_span.start();

                let is_deeper = new_start.line > old_start.line || (new_start.line == old_start.line && new_start.column > old_start.column);
                let is_shallower = old_start.line > new_start.line || (old_start.line == new_start.line && old_start.column > new_start.column);

                if is_deeper {
                    #[cfg(feature = "trace")]
                    eprintln!("[TRACE] record_error: New best error (deeper span)");
                    self.best_error = Some(new_error_state); 
                    return;
                } else if is_shallower {
                    return;
                }

                if new_error_state.priority >= existing.priority {
                    #[cfg(feature = "trace")]
                    eprintln!("[TRACE] record_error: New best error (higher/equal priority at same span)");
                    self.best_error = Some(new_error_state);
                } else {
                    #[cfg(feature = "trace")]
                    eprintln!("[TRACE] record_error: Rejected (lower priority at same span)");
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
            if original_err_str.contains("unexpected end of input") {
                msg = format!("unexpected end of input, expected {}", label);
            } else {
                msg = format!("expected {}", label);
            }
        } else {
            msg = original_err_str;
        }

        let line = best.start_span.start().line;
        let col = best.start_span.start().column;
        
        if !msg.contains(&format!("at column {}", col)) {
            msg = format!("{} at column {} (line {})", msg, col, line);
        }

        if !best.rule_stack.is_empty() {
            for rule in best.rule_stack.iter().rev() {
                let suffix = format!("\nin {}", rule);
                if !msg.contains(&suffix) { msg = format!("{}{}", msg, suffix); }
            }
        }

        Some(syn::Error::new(best.start_span, msg))
    }

    #[cfg(feature = "syn")]
    pub fn stop_aggregation(&self, current_span: Span) -> bool {
        if let Some(e) = &self.best_error {
            if e.is_fatal { return true; }
            if e.priority >= Self::PRIO_STRUCTURAL { return true; }

            let e_start = e.start_span.start();
            let c_start = current_span.start();
            if e_start.line > c_start.line || (e_start.line == c_start.line && e_start.column > c_start.column) {
                return true;
            }
        }
        false
    }

    pub fn enter_lexical(&mut self) { self.mode_stack.push(true); }
    pub fn enter_spaced(&mut self) { self.mode_stack.push(false); }
    pub fn exit_mode(&mut self) { self.mode_stack.pop(); }
    pub fn is_lexical(&self) -> bool { *self.mode_stack.last().unwrap_or(&false) }

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
        if let Some(last) = self.last_span { last.end() != next_span.start() } else { true }
    }

    pub fn enter_scope(&mut self) { self.scopes.enter_scope(); }
    pub fn exit_scope(&mut self) { self.scopes.exit_scope(); }
    pub fn define(&mut self, name: impl Into<String>) { self.scopes.define(name); }
    pub fn is_defined(&self, name: &str) -> bool { self.scopes.is_defined(name) }
    pub fn scopes(&self) -> &Vec<HashSet<String>> { self.scopes.scopes() }
    pub fn rule_stack(&self) -> &Vec<String> { &self.rule_stack }
}

#[cfg(feature = "rt")]
impl Default for ParseContext { fn default() -> Self { Self::new() } }

#[cfg(all(test, feature = "rt", feature = "syn"))]
mod tests {
    use super::*;
    use syn::parse::Parser;
    use syn::parse::ParseStream;

    #[test]
    fn test_rule_name_in_error() {
        let mut ctx = ParseContext::new();
        ctx.enter_rule("test_rule");
        let err = syn::Error::new(Span::call_site(), "expected something");
        ctx.record_error(err, Span::call_site(), None, 0);
        let final_err = ctx.take_best_error().unwrap();
        assert!(final_err.to_string().contains("expected something") && final_err.to_string().contains("in test_rule"));
    }

    #[test]
    fn test_attempt_captures_rule_context() {
        let mut ctx = ParseContext::new();
        let parser = |input: ParseStream| {
            ctx.enter_rule("outer");
            let _ = match crate::combinators::attempt(input, &mut ctx, |_input, _ctx| {
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
        assert!(err.to_string().contains("parse failed") && err.to_string().contains("in outer"));
    }
}
