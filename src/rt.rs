use syn::parse::ParseStream;
use syn::Result;

// 1. WICHTIG für Backtracking: Aktiviert .fork() und .advance_to()
use syn::parse::discouraged::Speculative;

// 2. WICHTIG für Identifier: Aktiviert .parse_any() (erlaubt Keywords als Namen)
use syn::ext::IdentExt; 

/// Kapselt einen spekulativen Parse-Versuch. 
/// Erlaubt es dem Generator, Backtracking als Einzeiler zu schreiben.
pub fn attempt<T>(input: ParseStream, parser: impl FnOnce(ParseStream) -> Result<T>) -> Result<Option<T>> {
    let fork = input.fork(); // Benötigt 'Speculative'
    match parser(&fork) {
        Ok(res) => {
            input.advance_to(&fork); // Benötigt 'Speculative'
            Ok(Some(res))
        }
        Err(_) => Ok(None),
    }
}

/// Helper für Identifier (erlaubt Keywords)
pub fn parse_ident(input: ParseStream) -> Result<syn::Ident> {
    // Benötigt 'IdentExt'
    input.call(syn::Ident::parse_any)
}

/// Helper für typisierte Integer
pub fn parse_int<T: std::str::FromStr>(input: ParseStream) -> Result<T> 
where T::Err: std::fmt::Display {
    input.parse::<syn::LitInt>()?.base10_parse()
}
