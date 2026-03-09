extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::parse::Parser;
use syn::{parse_macro_input, Fields, ItemStruct};

#[proc_macro_attribute]
pub fn with_span(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(item as ItemStruct);

    // 1. Add the span field to the struct
    if let Fields::Named(ref mut fields) = input.fields {
        fields.named.push(
            syn::Field::parse_named
                .parse2(quote! { pub span: std::ops::Range<usize> })
                .expect("Failed to parse span field"),
        );
    } else {
        return syn::Error::new_spanned(
            &input.fields,
            "with_span can only be used on structs with named fields",
        )
        .to_compile_error()
        .into();
    }

    let name = &input.ident;
    let (impl_generics, ty_generics, _where_clause) = input.generics.split_for_impl();

    // Determine the ParsedData type.
    // For now, we assume the user wants a generic implementation or we'd need more info.
    // However, the Trait WithSpan<ParsedData> is generic.
    // We'll implement it for the struct itself as the data source if it matches,
    // but typically it's used to map from a "Raw" version to the "AST" version.

    // A common pattern is: ParsedData is the same struct but without the span.
    // But since the macro modifies the struct, we implement it for 'Self'.

    let expanded = quote! {
        #input

        impl #impl_generics WithSpan<#name #ty_generics> for #name #ty_generics {
            fn with_span(mut parsed_data: Self, span: std::ops::Range<usize>) -> Self {
                parsed_data.span = span;
                parsed_data
            }
        }
    };

    TokenStream::from(expanded)
}
