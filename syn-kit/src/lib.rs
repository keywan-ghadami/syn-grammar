use syn::parse::ParseStream;
use syn::Result;
use proc_macro2::Span;
use syn::parse::discouraged::Speculative;
use syn::ext::IdentExt; 

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
}

impl ParseContext {
    pub fn new() -> Self {
        Self {
            is_fatal: false,
            best_error: None,
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
