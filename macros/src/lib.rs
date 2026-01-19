extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Data, Fields};
use proc_macro_error::{proc_macro_error, abort};

#[proc_macro_derive(ModelConvert)]
#[proc_macro_error]
pub fn derive_model_convert(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let fields = match &input.data {
        Data::Struct(d) => match &d.fields {
            Fields::Named(f) => &f.named,
            _ => abort!(input.ident, "Only named fields are supported"),
        },
        _ => abort!(input.ident, "Only structs are supported"),
    };

    let field_mappings = fields.iter().filter_map(|f| {
        let f_name = f.ident.as_ref().unwrap();
        let s_name = f_name.to_string();
        
        // 1. Überspringe Boilerplate-Felder (beginnen mit '_')
        if s_name.starts_with('_') {
            return None;
        }

        // 2. Spezialfälle (Logik für is_pub und inherits)
        if s_name == "is_pub" {
            return Some(quote! { is_pub: p.is_pub.is_some() });
        }
        if s_name == "inherits" {
            return Some(quote! { inherits: p.inherits.map(|i| i.name) });
        }

        // 3. Typ-basierte Entscheidung
        // Wir entscheiden anhand des Feldnamens, ob es eine Liste ist.
        // Listen müssen iteriert werden, Einzelwerte einfach konvertiert.
        let is_collection = matches!(s_name.as_str(), "rules" | "variants" | "pattern");

        if is_collection {
            // Für Vec<T>: map(Into::into).collect()
            Some(quote! { 
                #f_name: p.#f_name.into_iter().map(Into::into).collect() 
            })
        } else {
            // Für Ident, Type, TokenStream: einfaches .into()
            // Das behebt den Fehler "Ident is not an iterator"
            Some(quote! { 
                #f_name: p.#f_name.into() 
            })
        }
    });

    quote! {
        impl From<crate::parser::#name> for crate::model::#name {
            fn from(p: crate::parser::#name) -> Self {
                Self {
                    #(#field_mappings),*
                }
            }
        }
    }.into()
}

#[proc_macro]
#[proc_macro_error]
pub fn grammar(input: TokenStream) -> TokenStream {
    let input2 = proc_macro2::TokenStream::from(input);
    
    quote! {
        {
            let p_ast: crate::parser::GrammarDefinition = syn::parse_quote! { #input2 };
            let m_ast: crate::model::GrammarDefinition = p_ast.into();
            m_ast
        }
    }.into()
}
