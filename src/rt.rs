use syn::parse::{ParseBuffer, ParseStream};
use syn::Result;

/// Parsed einen Identifier, akzeptiert aber auch Rust-Keywords (z.B. "fn" als Name).
/// Ersetzt: input.call(syn::Ident::parse_any)
pub fn parse_ident(input: ParseStream) -> Result<syn::Ident> {
    input.call(syn::Ident::parse_any)
}

/// Parsed ein Integer-Literal in einen typisierten Wert (z.B. i32).
/// Ersetzt: input.parse::<LitInt>()?.base10_parse()
pub fn parse_int<T: std::str::FromStr>(input: ParseStream) -> Result<T>
where T::Err: std::fmt::Display {
    let lit = input.parse::<syn::LitInt>()?;
    lit.base10_parse()
}

/// Spekulatives Parsing (Backtracking).
/// Versucht `parser` auszuführen.
/// - Bei Erfolg: Konsumiert Tokens und gibt Some(Ergebnis) zurück.
/// - Bei Fehler: Konsumiert NICHTS und gibt Ok(None) zurück (Fehler wird verworfen).
/// 
/// Das vereinfacht den Generator massiv, da wir keine "let fork = ..." Strings mehr bauen müssen.
pub fn parse_speculative<T>(
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
            // Fehler ignorieren, Fork verwerfen
            Ok(None)
        }
    }
}
