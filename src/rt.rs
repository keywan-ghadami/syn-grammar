use syn::parse::ParseStream;
use syn::Result;

// 1. IMPORTANT for backtracking: Enables .fork() and .advance_to()
use syn::parse::discouraged::Speculative;

// 2. IMPORTANT for identifiers: Enables .parse_any() (allows keywords as names)
use syn::ext::IdentExt; 

/// Encapsulates a speculative parse attempt. 
/// Allows the generator to write backtracking as a one-liner.
pub fn attempt<T>(input: ParseStream, parser: impl FnOnce(ParseStream) -> Result<T>) -> Result<Option<T>> {
    let fork = input.fork(); // Requires 'Speculative'
    match parser(&fork) {
        Ok(res) => {
            input.advance_to(&fork); // Requires 'Speculative'
            Ok(Some(res))
        }
        Err(_) => Ok(None),
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
