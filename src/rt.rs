use syn::parse::ParseStream;
use syn::Result;
use std::cell::Cell;

// 1. IMPORTANT for backtracking: Enables .fork() and .advance_to()
use syn::parse::discouraged::Speculative;

// 2. IMPORTANT for identifiers: Enables .parse_any() (allows keywords as names)
use syn::ext::IdentExt; 

thread_local! {
    static IS_FATAL: Cell<bool> = const { Cell::new(false) };
}

pub fn set_fatal(fatal: bool) {
    IS_FATAL.set(fatal);
}

pub fn check_fatal() -> bool {
    IS_FATAL.get()
}

/// Encapsulates a speculative parse attempt. 
/// Allows the generator to write backtracking as a one-liner.
pub fn attempt<T>(input: ParseStream, parser: impl FnOnce(ParseStream) -> Result<T>) -> Result<Option<T>> {
    let was_fatal = check_fatal();
    set_fatal(false);

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
