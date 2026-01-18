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
        
        if s_name.starts_with('_') {
            None
        } else if s_name == "is_pub" {
            // Speziallogik für Sichtbarkeit (Option<Token![pub]> -> bool)
            Some(quote! { is_pub: p.is_pub.is_some() })
        } else if s_name == "inherits" {
            // Speziallogik für Vererbung
            Some(quote! { inherits: p.inherits.map(|i| i.name) })
        } else {
            // Standard: Rekursives .into()
            Some(quote! { #f_name: p.#f_name.into_iter().map(Into::into).collect() })
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
    // Wandelt den Input (TokenStream) in einen Parser-AST um und dann in ein Model
    quote! {
        {
            let p_ast: syn_grammar::parser::GrammarDefinition = syn::parse_quote! { #input };
            let m_ast: syn_grammar::model::GrammarDefinition = p_ast.into();
            m_ast
        }
    }.into()
}
