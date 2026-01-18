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
        
        // Felder, die mit '_' beginnen, sind Token-Boilerplate im Parser
        // und existieren nicht im sauberen Modell.
        if s_name.starts_with('_') {
            None
        } else if s_name == "is_pub" {
            // Konvertiert Option<Token![pub]> (Parser) zu bool (Model)
            Some(quote! { is_pub: p.is_pub.is_some() })
        } else if s_name == "inherits" {
            // Konvertiert Option<InheritanceSpec> zu Option<Ident>
            Some(quote! { inherits: p.inherits.map(|i| i.name) })
        } else {
            // Standard: Nutzt .into() (funktioniert für Vec<T> durch impl From für Pattern)
            Some(quote! { 
                #f_name: p.#f_name.into_iter().map(Into::into).collect() 
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
    // FIX: Konvertierung von proc_macro zu proc_macro2 für quote! Kompatibilität
    let input2 = proc_macro2::TokenStream::from(input);
    
    quote! {
        {
            // Nutzt syn::parse_quote, um den TokenStream zur Compile-Zeit des Tests
            // in einen Parser-AST zu wandeln und diesen dann zu konvertieren.
            let p_ast: crate::parser::GrammarDefinition = syn::parse_quote! { #input2 };
            let m_ast: crate::model::GrammarDefinition = p_ast.into();
            m_ast
        }
    }.into()
}
