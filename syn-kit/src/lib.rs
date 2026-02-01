use syn::parse::ParseStream;
use syn::Result;
use proc_macro2::Span;
use syn::parse::discouraged::Speculative;
use syn::ext::IdentExt; 
use std::collections::HashSet;

pub mod testing;

struct ErrorState {
    err: syn::Error,
    is_deep: bool,
}

/// Holds the state for backtracking and error reporting.
/// This must be passed mutably through the parsing chain.
pub struct ParseContext {
    is_fatal: bool,
    best_error: Option<ErrorState>,
    scopes: Vec<HashSet<String>>,
}

impl ParseContext {
    pub fn new() -> Self {
        Self {
            is_fatal: false,
            best_error: None,
            scopes: vec![HashSet::new()],
        }
    }

    pub fn set_fatal(&mut self, fatal: bool) {
        self.is_fatal = fatal;
    }

    pub fn check_fatal(&self) -> bool {
        self.is_fatal
    }

    /// Records an error if it is "deeper" than the current best error.
    pub fn record_error(&mut self, err: syn::Error, start_span: Span) {
        // Heuristic: Compare the error location to the start of the attempt.
        let is_deep = err.span().start() != start_span.start();

        match &mut self.best_error {
            None => {
                self.best_error = Some(ErrorState { err, is_deep });
            }
            Some(existing) => {
                // If new is deep and existing is shallow -> Overwrite
                if is_deep && !existing.is_deep {
                    self.best_error = Some(ErrorState { err, is_deep });
                }
            }
        }
    }

    pub fn take_best_error(&mut self) -> Option<syn::Error> {
        self.best_error.take().map(|s| s.err)
    }

    // --- Symbol Table Methods ---

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
}

impl Default for ParseContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Encapsulates a speculative parse attempt. 
/// Requires passing the ParseContext to manage error state.
#[inline]
pub fn attempt<T, F>(
    input: ParseStream, 
    ctx: &mut ParseContext, 
    parser: F
) -> Result<Option<T>> 
where 
    F: FnOnce(ParseStream, &mut ParseContext) -> Result<T>
{
    let was_fatal = ctx.check_fatal();
    ctx.set_fatal(false);

    // Snapshot symbol table
    let scopes_snapshot = ctx.scopes.clone();

    let start_span = input.span();
    let fork = input.fork();
    
    // Pass ctx into the closure
    let res = parser(&fork, ctx);
    
    let is_now_fatal = ctx.check_fatal();

    match res {
        Ok(val) => {
            input.advance_to(&fork);
            ctx.set_fatal(was_fatal);
            Ok(Some(val))
        }
        Err(e) => {
            // Restore symbol table on failure
            ctx.scopes = scopes_snapshot;

            if is_now_fatal {
                ctx.set_fatal(true);
                Err(e)
            } else {
                ctx.set_fatal(was_fatal);
                ctx.record_error(e, start_span);
                Ok(None)
            }
        }
    }
}

/// Wrapper around attempt used specifically for recovery blocks.
#[inline]
pub fn attempt_recover<T, F>(
    input: ParseStream, 
    ctx: &mut ParseContext,
    parser: F
) -> Result<Option<T>>
where 
    F: FnOnce(ParseStream, &mut ParseContext) -> Result<T>
{
    let was_fatal = ctx.check_fatal();
    ctx.set_fatal(false);

    // Snapshot symbol table
    let scopes_snapshot = ctx.scopes.clone();

    let start_span = input.span();
    let fork = input.fork();
    
    let res = parser(&fork, ctx);
    
    // Always restore fatal state, ignoring whatever happened inside.
    ctx.set_fatal(was_fatal);

    match res {
        Ok(val) => {
            input.advance_to(&fork);
            Ok(Some(val))
        }
        Err(e) => {
            // Restore symbol table on failure
            ctx.scopes = scopes_snapshot;
            
            ctx.record_error(e, start_span);
            Ok(None)
        }
    }
}

// --- Stateless Helpers (No Context Needed) ---

#[inline]
pub fn parse_ident(input: ParseStream) -> Result<syn::Ident> {
    input.call(syn::Ident::parse_any)
}

#[inline]
pub fn parse_int<T: std::str::FromStr>(input: ParseStream) -> Result<T> 
where T::Err: std::fmt::Display {
    input.parse::<syn::LitInt>()?.base10_parse()
}

pub fn skip_until(input: ParseStream, predicate: impl Fn(ParseStream) -> bool) -> Result<()> {
    while !input.is_empty() && !predicate(input) {
        if input.parse::<proc_macro2::TokenTree>().is_err() {
            break; 
        }
    }
    Ok(())
}
