use syn::parse::{ParseBuffer, ParseStream};
use syn::Result;

// --- Basic Parsers ---

pub fn parse_ident(input: ParseStream) -> Result<syn::Ident> {
    input.call(syn::Ident::parse_any)
}

pub fn parse_int<T: std::str::FromStr>(input: ParseStream) -> Result<T>
where T::Err: std::fmt::Display {
    let lit = input.parse::<syn::LitInt>()?;
    lit.base10_parse()
}

// --- Combinators ---

/// Versucht einen Parser auszuführen.
/// - Erfolg: Gibt `Ok(Some(T))` zurück und konsumiert Tokens.
/// - Fehler: Gibt `Ok(None)` zurück, setzt den Input zurück (Backtracking) und verwirft den Fehler.
/// - Fataler Fehler: Kann durchgeschleift werden, wenn wir Result<Option<T>> nutzen.
pub fn parse_try<T>(
    input: ParseStream,
    parser: impl Fn(ParseStream) -> Result<T>,
) -> Result<Option<T>> {
    let fork = input.fork();
    match parser(&fork) {
        Ok(result) => {
            input.advance_to(&fork);
            Ok(Some(result))
        }
        Err(_) => {
            // Fehler ignorieren (soft fail), Input nicht konsumieren
            Ok(None)
        }
    }
}

/// Auswahl (Alternative). Versucht erst A, wenn das fehlschlägt (soft), dann B.
/// Dies ist eine Runtime-Funktion, die wir nutzen könnten, wenn wir Closures generieren.
/// Aktuell generiert der Codegen aber if-else Kaskaden mit parse_try, was flexibler für Typen ist.
pub fn select<T>(
    input: ParseStream,
    attempts: &[fn(ParseStream) -> Result<Option<T>>]
) -> Result<T> {
    for attempt in attempts {
        if let Some(res) = attempt(input)? {
            return Ok(res);
        }
    }
    Err(input.error("No matching rule variant found"))
}
