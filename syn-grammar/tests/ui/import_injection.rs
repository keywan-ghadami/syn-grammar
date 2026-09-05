// Up to 0.8.0 a function brought in with `use` could be called like a rule
// ("import injection"). Since 0.9.0 hand-written parsers are declared with
// `extern rule`; the error must say so instead of a bare "Undefined rule".
use syn_grammar::grammar;

pub struct MyType;
pub fn my_custom_parser(input: syn::parse::ParseStream) -> syn::Result<MyType> {
    let _: syn::Ident = input.parse()?;
    Ok(MyType)
}

grammar! {
    grammar Inject {
        use super::my_custom_parser;

        pub rule main -> () = val:my_custom_parser -> { let _ = val; }
    }
}

fn main() {}
