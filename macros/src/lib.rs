extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Data, Fields};

#[proc_macro_derive(ModelConvert)]
pub fn derive_model_convert(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    let convert_logic = match input.data {
        Data::Struct(data) => {
            let fields = match data.fields {
                Fields::Named(f) => f.named,
                _ => panic!("Only named fields supported"),
            };

            let field_conv = fields.iter().map(|f| {
                let f_name = &f.ident;
                // Wir nehmen an: Felder mit Unterstrich sind Token-Boilerplate und werden ignoriert
                if f_name.as_ref().unwrap().to_string().starts_with('_') {
                    quote! {}
                } else {
                    quote! { #f_name: p.#f_name.into(), }
                }
            });

            quote! {
                impl From<crate::parser::#name> for crate::model::#name {
                    fn from(p: crate::parser::#name) -> Self {
                        Self {
                            #(#field_conv)*
                        }
                    }
                }
            }
        }
        _ => panic!("Only structs supported"),
    };

    convert_logic.into()
}

