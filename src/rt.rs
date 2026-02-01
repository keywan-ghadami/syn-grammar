use syn::parse::ParseStream;
use syn::Result;
use std::cell::{Cell, RefCell};

// 1. IMPORTANT for backtracking: Enables .fork() and .advance_to()
use syn::parse::discouraged::Speculative;

// 2. IMPORTANT for identifiers: Enables .parse_any() (allows keywords as names)
use syn::ext::IdentExt; 

thread_local! {
    static IS_FATAL: Cell<bool> = const { Cell::new(false) };
    static BEST_ERROR: RefCell<Option<syn::Error>> = RefCell::new(None);
}

pub fn set_fatal(fatal: bool) {
    IS_FATAL.set(fatal);
}

pub fn check_fatal() -> bool {
    IS_FATAL.get()
}

fn record_error(err: syn::Error, start_span_debug: String) {
    BEST_ERROR.with(|cell| {
        let mut borrow = cell.borrow_mut();
        
        // Heuristic: Compare the error location to the start of the attempt.
        // If they differ, we made progress (Deep Error).
        // We prioritize Deep Errors over Shallow Errors.
        let err_span_debug = format!("{:?}", err.span());
        let is_deep = err_span_debug != start_span_debug;

        match &*borrow {
            None => {
                *borrow = Some(err);
            }
            Some(existing) => {
                // If the new error is Deep, we prefer it.
                // A more sophisticated check might compare actual line/column if available,
                // but checking inequality with start is a good proxy for "moved forward".
                if is_deep {
                    *borrow = Some(err);
                }
            }
        }
    });
}

pub fn take_best_error() -> Option<syn::Error> {
    BEST_ERROR.with(|cell| cell.borrow_mut().take())
}

/// Encapsulates a speculative parse attempt. 
/// Allows the generator to write backtracking as a one-liner.
pub fn attempt<T>(input: ParseStream, parser: impl FnOnce(ParseStream) -> Result<T>) -> Result<Option<T>> {
    let was_fatal = check_fatal();
    set_fatal(false);

    let start_span = format!("{:?}", input.span());

    let fork = input.fork(); // Requires 'Speculative'
    let res = parser(&fork);
    
    let is_now_fatal = check_fatal();

    match res {
        Ok(val) => {
            input.advance_to(&fork); // Requires 'Speculative'
            set_fatal(was_fatal);
            Ok(Some(val))
        }
        Err(e) => {
            if is_now_fatal {
                set_fatal(true);
                Err(e)
            } else {
                set_fatal(was_fatal);
                record_error(e, start_span);
                Ok(None)
            }
        }
    }
}

/// Helper for identifiers (allows keywords)
pub fn parse_ident(input: ParseStream) -> Result<syn::Ident> {
    // Requires 'IdentExt'
    input.call(syn::Ident::parse_any)
}

/// Helper for typed integers
pub fn parse_int<T: std::str::FromStr>(input: ParseStream) -> Result<T> 
where T::Err: std::fmt::Display {
    input.parse::<syn::LitInt>()?.base10_parse()
}

/// Skips tokens until the predicate returns true or input is empty.
pub fn skip_until(input: ParseStream, predicate: impl Fn(ParseStream) -> bool) -> Result<()> {
    while !input.is_empty() && !predicate(input) {
        if input.parse::<proc_macro2::TokenTree>().is_err() {
            break; 
        }
    }
    Ok(())
}

/// Wrapper around attempt used specifically for recovery blocks.
/// It allows the generated code to distinguish semantic intent, 
/// though currently it shares the same backtracking logic.
pub fn attempt_recover<T>(
    input: ParseStream, 
    parser: impl FnOnce(ParseStream) -> Result<T>
) -> Result<Option<T>> {
    attempt(input, parser)
}
