use crate::model::*;
use quote::{quote, quote_spanned, format_ident};
use proc_macro2::TokenStream;
use std::collections::HashSet;
use syn::Result;

pub fn generate_rust(grammar: GrammarDefinition) -> Result<TokenStream> {
    let mut output = TokenStream::new();
    let grammar_name = &grammar.name;
    let custom_keywords = collect_custom_keywords(&grammar);

    output.extend(quote! {
        #![allow(unused_imports, unused_variables, dead_code)]
        pub const GRAMMAR_NAME: &str = stringify!(#grammar_name);
        use syn::parse::{Parse, ParseStream};
        use syn::Result;
        use syn::Token;
        use syn::ext::IdentExt; 
        use syn_grammar::rt; 
    });

    if !custom_keywords.is_empty() {
        let kw_defs = custom_keywords.iter().map(|k| {
            let ident = format_ident!("{}", k);
            quote! { syn::custom_keyword!(#ident); }
        });
        output.extend(quote! {
            pub mod kw { #(#kw_defs)* }
        });
    }

    if let Some(parent) = &grammar.inherits {
        output.extend(quote! { use super::#parent::*; });
    }

    for rule in &grammar.rules {
        output.extend(generate_rule(rule, &custom_keywords)?);
    }

    Ok(output)
}

fn generate_rule(rule: &Rule, custom_keywords: &HashSet<String>) -> Result<TokenStream> {
    let name = &rule.name;
    let fn_name = format_ident!("parse_{}", name);
    let ret_type = &rule.return_type;
    let vis = if rule.is_pub { quote!(pub) } else { quote!() };

    let body = generate_variants(&rule.variants, true, custom_keywords)?; 

    Ok(quote! {
        #vis fn #fn_name(input: ParseStream) -> Result<#ret_type> {
            #body
        }
    })
}

fn generate_variants(
    variants: &[RuleVariant], 
    is_top_level: bool,
    custom_keywords: &HashSet<String>
) -> Result<TokenStream> {
    // Fallback Code (Ende der Kette)
    let mut current_code = if is_top_level {
        quote! { Err(input.error("No matching rule variant found")) }
    } else {
        quote! { Err(input.error("No matching variant in group")) }
    };

    // Wir bauen die Kette rückwärts
    for (i, variant) in variants.iter().enumerate().rev() {
        let logic = generate_sequence(&variant.pattern, &variant.action, custom_keywords)?;
        
        // Optimierung: Einfacher Lookahead
        // Wir prüfen NUR das allererste Element des Patterns.
        // Wenn es ein Literal oder eine Klammer ist, können wir peeken.
        // Wenn es ein RuleCall ist, wissen wir es nicht -> Speculative Fallback.
        let peek_token = if let Some(first) = variant.pattern.first() {
            get_peek_token(first, custom_keywords)?
        } else {
            None
        };

        // Wenn wir in der letzten Variante Top-Level sind, brauchen wir keinen Check,
        // wir probieren es einfach (oder failen).
        if i == variants.len() - 1 && is_top_level {
            current_code = logic;
            continue;
        }

        if let Some(token) = peek_token {
            // LL(1) Optimierung
            current_code = quote! {
                if input.peek(#token) {
                    #logic
                } else {
                    #current_code
                }
            };
        } else {
            // Robuster Fallback: Speculative Parsing via Runtime
            current_code = quote! {
                if let Some(res) = rt::parse_speculative(input, |input| {
                    #logic
                })? {
                    res
                } else {
                    #current_code
                }
            };
        }
    }

    Ok(current_code)
}

fn generate_sequence(patterns: &[Pattern], action: &TokenStream, kws: &HashSet<String>) -> Result<TokenStream> {
    let mut steps = TokenStream::new();
    for pattern in patterns {
        steps.extend(generate_pattern_step(pattern, kws)?);
    }
    Ok(quote! {
        {
            #steps
            Ok(#action)
        }
    })
}

fn generate_pattern_step(pattern: &Pattern, kws: &HashSet<String>) -> Result<TokenStream> {
    let span = pattern.span();
    match pattern {
        Pattern::Lit(lit) => {
            let token_type = resolve_token_type(lit, kws)?;
            Ok(quote_spanned! {span=> 
                let _ = input.parse::<#token_type>()?; 
            })
        },
        Pattern::RuleCall { binding, rule_name, args } => {
            let func_call = if is_builtin(rule_name) {
                map_builtin(rule_name)
            } else {
                let f = format_ident!("parse_{}", rule_name);
                if args.is_empty() {
                    quote! { #f(input)? }
                } else {
                    quote! { #f(input, #(#args),*)? }
                }
            };
            
            if let Some(bind) = binding {
                Ok(quote_spanned! {span=> let #bind = #func_call; })
            } else {
                Ok(quote_spanned! {span=> let _ = #func_call; })
            }
        },
        Pattern::Optional(inner) => {
            let inner_logic = generate_pattern_step(inner, kws)?;
            // Optional braucht Peek! Wenn wir nicht peeken können, ist das Pattern ungültig.
            let peek = get_peek_token(inner, kws)?.ok_or_else(|| 
                syn::Error::new(span, "Optional pattern must start with a literal or token"))?;
            
            Ok(quote_spanned! {span=> if input.peek(#peek) { #inner_logic } })
        },
        Pattern::Repeat(inner) => {
             let inner_logic = generate_pattern_step(inner, kws)?;
             let peek = get_peek_token(inner, kws)?.ok_or_else(|| 
                syn::Error::new(span, "Repeat pattern must start with a literal or token"))?;

             Ok(quote_spanned! {span=> while input.peek(#peek) { #inner_logic } })
        },
        Pattern::Plus(inner) => {
            let inner_logic = generate_pattern_step(inner, kws)?;
            let peek = get_peek_token(inner, kws)?.ok_or_else(|| 
                syn::Error::new(span, "Plus pattern must start with a literal or token"))?;

             Ok(quote_spanned! {span=>
                if !input.peek(#peek) { return Err(input.error("Expected at least one occurrence")); }
                while input.peek(#peek) { #inner_logic }
             })
        },
        Pattern::Group(alts) => {
            let temp_variants: Vec<RuleVariant> = alts.iter().map(|pat_seq| {
                RuleVariant { pattern: pat_seq.clone(), action: quote!({}) }
            }).collect();
            let variant_logic = generate_variants(&temp_variants, false, kws)?;
            Ok(quote_spanned! {span=> { #variant_logic }?; })
        },
        Pattern::Bracketed(seq) => {
            let inner_logic = generate_sequence_no_action(seq, kws)?;
            Ok(quote_spanned! {span=>
                let content;
                let _ = syn::bracketed!(content in input);
                let input = &content; 
                #inner_logic
            })
        },
        Pattern::Braced(seq) => {
            let inner_logic = generate_sequence_no_action(seq, kws)?;
            Ok(quote_spanned! {span=>
                let content;
                let _ = syn::braced!(content in input);
                let input = &content;
                #inner_logic
            })
        },
        Pattern::Parenthesized(seq) => {
            let inner_logic = generate_sequence_no_action(seq, kws)?;
            Ok(quote_spanned! {span=>
                let content;
                let _ = syn::parenthesized!(content in input);
                let input = &content;
                #inner_logic
            })
        },
    }
}

fn generate_sequence_no_action(patterns: &[Pattern], kws: &HashSet<String>) -> Result<TokenStream> {
    let mut steps = TokenStream::new();
    for pattern in patterns {
        steps.extend(generate_pattern_step(pattern, kws)?);
    }
    Ok(steps)
}

// --- Helpers ---

// Bestimmt das Start-Token für einfache Patterns. Gibt None zurück für komplexe/rekursive Patterns.
fn get_peek_token(pattern: &Pattern, kws: &HashSet<String>) -> Result<Option<TokenStream>> {
    match pattern {
        Pattern::Lit(lit) => {
            let t = resolve_token_type(lit, kws)?;
            Ok(Some(quote!(#t)))
        },
        Pattern::Bracketed(_) => Ok(Some(quote!(syn::token::Bracket))),
        Pattern::Braced(_) => Ok(Some(quote!(syn::token::Brace))),
        Pattern::Parenthesized(_) => Ok(Some(quote!(syn::token::Paren))),
        // Für RuleCall, Group etc. geben wir None zurück -> Fallback auf Speculative Parsing
        _ => Ok(None)
    }
}

fn resolve_token_type(lit: &syn::LitStr, custom_keywords: &HashSet<String>) -> Result<syn::Type> {
    let s = lit.value();
    if matches!(s.as_str(), "(" | ")" | "[" | "]" | "{" | "}") {
         return Err(syn::Error::new(lit.span(), "Invalid usage of delimiter as literal"));
    }
    if custom_keywords.contains(&s) {
        let ident = format_ident!("{}", s);
        return Ok(syn::parse_quote!(kw::#ident));
    }
    let type_str = format!("Token![{}]", s);
    syn::parse_str::<syn::Type>(&type_str).map_err(|_| 
        syn::Error::new(lit.span(), format!("Invalid token literal: '{}'", s))
    )
}

fn collect_custom_keywords(grammar: &GrammarDefinition) -> HashSet<String> {
    let mut kws = HashSet::new();
    for rule in &grammar.rules {
        for variant in &rule.variants {
            collect_from_patterns(&variant.pattern, &mut kws);
        }
    }
    kws
}

fn collect_from_patterns(patterns: &[Pattern], kws: &mut HashSet<String>) {
    for p in patterns {
        match p {
            Pattern::Lit(lit) => {
                let s = lit.value();
                if is_identifier(&s) && !is_rust_keyword(&s) {
                    kws.insert(s);
                }
            },
            Pattern::Group(alts) => {
                for alt in alts { collect_from_patterns(alt, kws); }
            },
            Pattern::Bracketed(seq) | Pattern::Braced(seq) | Pattern::Parenthesized(seq) => {
                collect_from_patterns(seq, kws);
            },
            Pattern::Optional(inner) | Pattern::Repeat(inner) | Pattern::Plus(inner) => {
                collect_from_patterns(&[ *inner.clone() ], kws); 
            },
            _ => {}
        }
    }
}

fn is_identifier(s: &str) -> bool {
    if s.is_empty() { return false; }
    let mut chars = s.chars();
    let first = chars.next().unwrap();
    if !first.is_alphabetic() && first != '_' { return false; }
    chars.all(|c| c.is_alphanumeric() || c == '_')
}

fn is_rust_keyword(s: &str) -> bool {
    matches!(s, 
        "fn" | "let" | "struct" | "enum" | "if" | "else" | "while" | "loop" | "for" | 
        "match" | "return" | "break" | "continue" | "pub" | "mod" | "use" | "type" | 
        "trait" | "impl" | "const" | "static" | "mut" | "unsafe" | "extern" | "ref" | 
        "self" | "Self" | "super" | "crate" | "async" | "await" | "where" | "move" | 
        "true" | "false" | "in" | "as" | "dyn" | "abstract" | "become" | "box" | "do" | 
        "final" | "macro" | "override" | "priv" | "typeof" | "unsized" | "virtual" | "yield"
    )
}

fn is_builtin(name: &syn::Ident) -> bool {
    matches!(name.to_string().as_str(), "ident" | "int_lit" | "string_lit")
}

fn map_builtin(name: &syn::Ident) -> TokenStream {
    match name.to_string().as_str() {
        "ident" => quote! { rt::parse_ident(input)? },
        "int_lit" => quote! { rt::parse_int::<i32>(input)? },
        "string_lit" => quote! { input.parse::<syn::LitStr>()?.value() },
        _ => panic!("Unknown builtin"),
    }
}
