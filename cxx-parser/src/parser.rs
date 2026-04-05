use syn::{Generics, ReturnType, Result};
use syn::parse::{ParseStream};

pub fn parse_generics(input: ParseStream) -> Result<Generics> {
    input.parse()
}

pub fn parse_return_type(input: ParseStream) -> Result<ReturnType> {
    input.parse()
}
