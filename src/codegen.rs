use crate::model::*;
use quote::{quote, quote_spanned, format_ident};
use proc_macro2::TokenStream;
use std::collections::{HashMap, HashSet};

pub fn generate_rust(grammar: GrammarDefinition) -> TokenStream {
    let mut output = TokenStream::new();
    let grammar_name = &grammar.name;
    let custom_keywords = collect_custom_keywords(&grammar);

    output.extend(quote! {
        #![allow(unused_imports, unused_variables, dead_code)]
        
        /// Auto-generated parser for grammar: #grammar_name
        pub const GRAMMAR_NAME: &str = stringify!(#grammar_name);

        use syn::parse::{Parse, ParseStream};
        use syn::Result;
        use syn::Token;
        use syn::ext::IdentExt; 
        
        // Wir nutzen jetzt unsere Runtime Library!
        // Annahme: Das generierte Crate hat Zugriff auf 'syn_grammar'.
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

    let first_sets = FirstSetComputer::new(&grammar, &custom_keywords);

    for rule in &grammar.rules {
        output.extend(generate_rule(rule, &first_sets, &custom_keywords));
    }

    output
}

fn generate_rule(rule: &Rule, first_sets: &FirstSetComputer, custom_keywords: &HashSet<String>) -> TokenStream {
    let name = &rule.name;
    let fn_name = format_ident!("parse_{}", name);
    let ret_type = &rule.return_type;
    
    // VISIBILITY FIX: Wenn die Regel public ist, generieren wir "pub"
    let vis = if rule.is_pub { quote!(pub) } else { quote!() };

    let body = generate_variants(&rule.variants, first_sets, true, custom_keywords); 

    quote! {
        #vis fn #fn_name(input: ParseStream) -> Result<#ret_type> {
            #body
        }
    }
}

fn generate_variants(
    variants: &[RuleVariant], 
    first_sets: &FirstSetComputer,
    is_top_level: bool,
    custom_keywords: &HashSet<String>
) -> TokenStream {
    let all_firsts: Vec<FirstSet> = variants.iter()
        .map(|v| first_sets.compute_sequence(&v.pattern))
        .collect();

    let mut current_code = if is_top_level {
        quote! { Err(input.error("No matching rule variant found")) }
    } else {
        quote! { Err(input.error("No matching variant in group")) }
    };

    for (i, variant) in variants.iter().enumerate().rev() {
        let logic = generate_sequence(&variant.pattern, &variant.action, first_sets, custom_keywords);
        let current_first = &all_firsts[i];

        let mut is_ambiguous = false;
        for other_first in all_firsts.iter().skip(i + 1) {
            if current_first.overlaps(other_first) {
                is_ambiguous = true;
                break;
            }
        }

        let can_peek = current_first.to_peek_check().is_some() && !is_ambiguous;

        if i == variants.len() - 1 && is_top_level {
            current_code = logic;
            continue;
        }

        if can_peek {
            let check = current_first.to_peek_check().unwrap();
            current_code = quote! {
                if #check {
                    #logic
                } else {
                    #current_code
                }
            };
        } else {
            // RUNTIME FIX: Statt fork code zu generieren, rufen wir rt::parse_speculative auf.
            // Das ist sauberer, kürzer und typsicherer.
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

    current_code
}

fn generate_sequence(patterns: &[Pattern], action: &TokenStream, first_sets: &FirstSetComputer, kws: &HashSet<String>) -> TokenStream {
    let mut steps = TokenStream::new();
    for pattern in patterns {
        steps.extend(generate_pattern_step(pattern, first_sets, kws));
    }
    quote! {
        {
            #steps
            Ok(#action)
        }
    }
}

fn generate_pattern_step(pattern: &Pattern, first_sets: &FirstSetComputer, kws: &HashSet<String>) -> TokenStream {
    let span = pattern.span();
    match pattern {
        Pattern::Lit(lit) => {
            let token_type = resolve_token_type(lit, kws);
            quote_spanned! {span=> 
                let _ = input.parse::<#token_type>()?; 
            }
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
                quote_spanned! {span=> let #bind = #func_call; }
            } else {
                quote_spanned! {span=> let _ = #func_call; }
            }
        },
        Pattern::Optional(inner) => {
            let inner_logic = generate_pattern_step(inner, first_sets, kws);
            let first = first_sets.compute_first(inner);
            if let Some(check) = first.to_peek_check() {
                quote_spanned! {span=> if #check { #inner_logic } }
            } else {
                 quote_spanned! {span=> }
            }
        },
        Pattern::Repeat(inner) => {
             let inner_logic = generate_pattern_step(inner, first_sets, kws);
             let first = first_sets.compute_first(inner);
             if let Some(check) = first.to_peek_check() {
                 quote_spanned! {span=> while #check { #inner_logic } }
             } else {
                 quote!() 
             }
        },
        Pattern::Plus(inner) => {
            let inner_logic = generate_pattern_step(inner, first_sets, kws);
            let first = first_sets.compute_first(inner);
             if let Some(check) = first.to_peek_check() {
                 quote_spanned! {span=>
                    if !#check { return Err(input.error("Expected at least one occurrence")); }
                    while #check { #inner_logic }
                 }
             } else {
                 quote!()
             }
        },
        Pattern::Group(alts) => {
            let temp_variants: Vec<RuleVariant> = alts.iter().map(|pat_seq| {
                RuleVariant { pattern: pat_seq.clone(), action: quote!({}) }
            }).collect();
            let variant_logic = generate_variants(&temp_variants, first_sets, false, kws);
            quote_spanned! {span=> { #variant_logic }?; }
        },
        Pattern::Bracketed(seq) => {
            let inner_logic = generate_sequence_no_action(seq, first_sets, kws);
            quote_spanned! {span=>
                let content;
                let _ = syn::bracketed!(content in input);
                let input = &content; 
                #inner_logic
            }
        },
        Pattern::Braced(seq) => {
            let inner_logic = generate_sequence_no_action(seq, first_sets, kws);
            quote_spanned! {span=>
                let content;
                let _ = syn::braced!(content in input);
                let input = &content;
                #inner_logic
            }
        },
        Pattern::Parenthesized(seq) => {
            let inner_logic = generate_sequence_no_action(seq, first_sets, kws);
            quote_spanned! {span=>
                let content;
                let _ = syn::parenthesized!(content in input);
                let input = &content;
                #inner_logic
            }
        },
    }
}

fn generate_sequence_no_action(patterns: &[Pattern], first_sets: &FirstSetComputer, kws: &HashSet<String>) -> TokenStream {
    let mut steps = TokenStream::new();
    for pattern in patterns {
        steps.extend(generate_pattern_step(pattern, first_sets, kws));
    }
    steps
}


fn resolve_token_type(lit: &syn::LitStr, custom_keywords: &HashSet<String>) -> syn::Type {
    let s = lit.value();
    
    if matches!(s.as_str(), "(" | ")" | "[" | "]" | "{" | "}") {
         panic!("Invalid usage of delimiter '{}' as a literal token. Use structural syntax like [ ... ] or {{ ... }} in your grammar.", s);
    }

    if custom_keywords.contains(&s) {
        let ident = format_ident!("{}", s);
        return syn::parse_quote!(kw::#ident);
    }

    let type_str = format!("Token![{}]", s);
    syn::parse_str::<syn::Type>(&type_str)
        .unwrap_or_else(|_| panic!("Invalid token literal: '{}'. Not a Rust keyword and not an identifier.", s))
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
        // RUNTIME FIX: Aufrufe an syn_grammar::rt
        "ident" => quote! { rt::parse_ident(input)? },
        "int_lit" => quote! { rt::parse_int::<i32>(input)? },
        "string_lit" => quote! { input.parse::<syn::LitStr>()?.value() },
        _ => panic!("Unknown builtin"),
    }
}

// --- First Set Analysis (Unverändert) ---

struct FirstSetComputer<'a> {
    rules: HashMap<String, &'a Rule>,
    custom_keywords: &'a HashSet<String>,
}

impl<'a> FirstSetComputer<'a> {
    fn new(grammar: &'a GrammarDefinition, kws: &'a HashSet<String>) -> Self {
        let mut rules = HashMap::new();
        for r in &grammar.rules {
            rules.insert(r.name.to_string(), r);
        }
        Self { rules, custom_keywords: kws }
    }

    fn compute_sequence(&self, patterns: &[Pattern]) -> FirstSet {
        let mut visited = HashSet::new();
        if let Some(first) = patterns.first() {
            self.compute_first_recursive(first, &mut visited)
        } else {
            FirstSet::Unknown 
        }
    }

    fn compute_first(&self, pattern: &Pattern) -> FirstSet {
        let mut visited = HashSet::new();
        self.compute_first_recursive(pattern, &mut visited)
    }

    fn compute_first_recursive(&self, pattern: &Pattern, visited: &mut HashSet<String>) -> FirstSet {
        match pattern {
            Pattern::Lit(l) => FirstSet::Token(resolve_token_type(l, self.custom_keywords)),
            
            Pattern::Bracketed(_) => FirstSet::Token(syn::parse_quote!(syn::token::Bracket)),
            Pattern::Braced(_) => FirstSet::Token(syn::parse_quote!(syn::token::Brace)),
            Pattern::Parenthesized(_) => FirstSet::Token(syn::parse_quote!(syn::token::Paren)),
            
            Pattern::RuleCall { rule_name, .. } => {
                if is_builtin(rule_name) {
                    return match rule_name.to_string().as_str() {
                        "ident" => FirstSet::Raw(quote!(syn::Ident).to_string()),
                        "int_lit" => FirstSet::Raw(quote!(syn::LitInt).to_string()),
                        "string_lit" => FirstSet::Raw(quote!(syn::LitStr).to_string()),
                        _ => FirstSet::Unknown
                    };
                }
                
                let rule_key = rule_name.to_string();
                if visited.contains(&rule_key) {
                    return FirstSet::Unknown;
                }
                visited.insert(rule_key.clone());

                let Some(rule) = self.rules.get(&rule_key) else { return FirstSet::Unknown; };
                let Some(first_var) = rule.variants.first() else { return FirstSet::Unknown; };
                let Some(first_pat) = first_var.pattern.first() else { return FirstSet::Unknown; };

                let result = self.compute_first_recursive(first_pat, visited);
                visited.remove(&rule_key); 
                result
            },
            Pattern::Group(alts) => {
                if let Some(first_alt) = alts.first() {
                    if let Some(p) = first_alt.first() {
                        self.compute_first_recursive(p, visited)
                    } else {
                        FirstSet::Unknown
                    }
                } else {
                    FirstSet::Unknown
                }
            },
            Pattern::Optional(inner) | Pattern::Repeat(inner) | Pattern::Plus(inner) => {
                self.compute_first_recursive(inner, visited)
            },
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum FirstSet {
    Token(syn::Type),
    Raw(String), 
    Unknown,
}

impl FirstSet {
    fn to_peek_check(&self) -> Option<TokenStream> {
        match self {
            FirstSet::Token(t) => Some(quote! { input.peek(#t) }),
            FirstSet::Raw(s) => {
                let t: syn::Type = syn::parse_str(s).ok()?;
                Some(quote! { input.peek(#t) })
            },
            FirstSet::Unknown => None,
        }
    }

    fn overlaps(&self, other: &FirstSet) -> bool {
        match (self, other) {
            (FirstSet::Unknown, _) | (_, FirstSet::Unknown) => true,
            (FirstSet::Raw(a), FirstSet::Raw(b)) => a == b,
            (FirstSet::Token(a), FirstSet::Token(b)) => {
                quote!(#a).to_string() == quote!(#b).to_string()
            }
            _ => false
        }
    }
}
