// syn-grammar/src/codegen.rs
use crate::model::*;
use quote::{quote, format_ident};
use proc_macro2::TokenStream;

pub fn generate_rust(grammar: GrammarDefinition) -> TokenStream {
    let mut output = TokenStream::new();
    
    // 1. Imports generieren (syn ist Pflicht)
    output.extend(quote! {
        use syn::parse::{Parse, ParseStream};
        use syn::Result;
        use syn::Token;
    });

    // 2. Regeln generieren
    for rule in grammar.rules {
        output.extend(generate_rule(rule));
    }

    output
}

fn generate_rule(rule: Rule) -> TokenStream {
    let name = &rule.name; // z.B. "fn_item"
    let fn_name = format_ident!("parse_{}", name); // "parse_fn_item"
    let ret_type = &rule.return_type; // "Item"
    
    let body = generate_variants(&rule.variants);

    // Wir generieren eine Funktion statt Trait-Impl, um flexibler zu sein
    let vis = if rule.is_pub { quote!(pub) } else { quote!() };

    quote! {
        #vis fn #fn_name(input: ParseStream) -> Result<#ret_type> {
            #body
        }
    }
}

fn generate_variants(variants: &[RuleVariant]) -> TokenStream {
    // Wenn es nur eine Variante gibt: Einfach generieren
    if variants.len() == 1 {
        return generate_sequence(&variants[0]);
    }

    // Wenn es Alternativen (|) gibt, müssen wir 'input.fork()' nutzen,
    // um zu schauen, welcher Pfad passt (Backtracking light).
    // Für Stage 0 vereinfachen wir: Wir nutzen input.peek().
    // (Ein echter Generator würde hier FIRST-Sets berechnen)
    
    let mut checks = TokenStream::new();
    
    for variant in variants {
        let parsing_logic = generate_sequence(variant);
        
        // Optimierung: Wenn das Pattern mit einem Token beginnt, generieren wir ein Peek
        if let Some(first_token) = find_first_token(&variant.pattern) {
             checks.extend(quote! {
                 if input.peek(#first_token) {
                     return { #parsing_logic };
                 }
             });
        } else {
             // Fallback: Einfach versuchen (könnte Fehler werfen)
             checks.extend(quote! {
                 // Hier bräuchte man echtes Speculative Parsing (Forking)
                 // Für den Prototyp lassen wir das user-seitig via 'peek' Regeln lösen.
             });
        }
    }
    
    quote! {
        #checks
        Err(input.error("No matching rule variant found"))
    }
}

fn generate_sequence(variant: &RuleVariant) -> TokenStream {
    let mut steps = TokenStream::new();
    let action = &variant.action; // Der { ... } Block aus der Grammatik

    for pattern in &variant.pattern {
        match pattern {
            Pattern::Lit(s) => {
                // "fn" -> input.parse::<Token![fn]>()?;
                let token_ident = map_literal_to_token(s); 
                steps.extend(quote! { let _ = input.parse::<#token_ident>()?; });
            },
            Pattern::RuleCall { binding, rule_name, .. } => {
                // name:ident() -> let name = input.parse::<Ident>()?;
                // oder: let name = parse_other_rule(input)?;
                
                let parse_call = if is_builtin(rule_name) {
                    map_builtin(rule_name) // ident() -> input.parse::<Ident>()?
                } else {
                    let func = format_ident!("parse_{}", rule_name);
                    quote! { #func(input)? }
                };

                if let Some(bind) = binding {
                    steps.extend(quote! { let #bind = #parse_call; });
                } else {
                    steps.extend(quote! { let _ = #parse_call; });
                }
            },
            // ... Repeat & Optional Logik ...
            _ => {}
        }
    }

    // Am Ende den Action-Block ausführen
    quote! {
        #steps
        Ok(#action)
    }
}

// --- Built-in Mapping (Die Standardbibliothek des Parsers) ---

fn is_builtin(name: &syn::Ident) -> bool {
    let s = name.to_string();
    matches!(s.as_str(), "ident" | "int_lit" | "string_lit")
}

fn map_builtin(name: &syn::Ident) -> TokenStream {
    match name.to_string().as_str() {
        "ident" => quote! { input.parse::<syn::Ident>()? },
        "int_lit" => quote! { input.parse::<syn::LitInt>()?.base10_parse()? },
        "string_lit" => quote! { input.parse::<syn::LitStr>()?.value() },
        _ => panic!("Unknown builtin"),
    }
}

fn map_literal_to_token(lit: &syn::LitStr) -> TokenStream {
    // Mapping von "fn" -> Token![fn]
    // Das ist etwas tricky, da man String -> Type mappen muss.
    // In Stage 0 hardcoden wir die wichtigsten Keywords.
    let s = lit.value();
    match s.as_str() {
        "fn" => quote!(Token![fn]),
        "let" => quote!(Token![let]),
        "struct" => quote!(Token![struct]),
        "=" => quote!(Token![=]),
        ";" => quote!(Token![;]),
        // ...
        _ => quote!(compile_error!(concat!("Unknown token: ", #s)))
    }
}

