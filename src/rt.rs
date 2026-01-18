pub struct ParserContext<'a> {
    input: &'a syn::parse::ParseBuffer<'a>,
}

impl<'a> ParserContext<'a> {
    pub fn new(input: &'a syn::parse::ParseBuffer<'a>) -> Self {
        Self { input }
    }

    /// Deklaratives Backtracking
    pub fn try_parse<T>(&self, f: impl Fn(syn::parse::ParseStream) -> syn::Result<T>) -> syn::Result<Option<T>> {
        let fork = self.input.fork();
        match f(&fork) {
            Ok(res) => {
                self.input.advance_to(&fork);
                Ok(Some(res))
            }
            Err(_) => Ok(None),
        }
    }
}
