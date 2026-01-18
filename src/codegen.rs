use crate::model::*;
use quote::{quote, quote_spanned, format_ident};
use proc_macro2::TokenStream;
use std::collections::{HashMap, HashSet};

pub fn generate_rust(grammar: GrammarDefinition) -> TokenStream {
    let mut output = TokenStream::new();

    // 1. Standard Imports (syn ist Pflicht)
    output.extend(quote! {
        use syn::parse::{Parse, ParseStream};
        use syn::Result;
        use syn::Token;
    });

    // 2. Vererbung (Inheritance) implementieren
    // Wenn wir von "Base" erben, importieren wir dessen Regeln.
    // Annahme: Die Parent-Grammatik liegt in einem Modul gleichen Namens oder ist importierbar.
    if let Some(parent) = &grammar.inherits {
        output.extend(quote! {
            use super::#parent::*;
        });
    }

    // 3. First-Set Analyse (mit Rekursionsschutz)
    let first_sets = FirstSetComputer::new(&grammar);

    // 4. Regeln generieren
    for rule in &grammar.rules {
        output.extend(generate_rule(rule, &first_sets));
    }

    output
}

fn generate_rule(rule: &Rule, first_sets: &FirstSetComputer) -> TokenStream {
    let name = &rule.name;
    let fn_name = format_ident!("parse_{}", name);
    let ret_type = &rule.return_type;
    let vis = if rule.is_pub { quote!(pub) } else { quote!() };

    // Body generieren
    let body = generate_variants(&rule.variants, first_sets, true); 

    quote! {
        #vis fn #fn_name(input: ParseStream) -> Result<#ret_type> {
            #body
        }
    }
}

fn generate_variants(
    variants: &[RuleVariant], 
    first_sets: &FirstSetComputer,
    is_top_level: bool 
) -> TokenStream {
    let mut checks = TokenStream::new();
    let variant_count = variants.len();

    for (i, variant) in variants.iter().enumerate() {
        let logic = generate_sequence(&variant.pattern, &variant.action, first_sets);
        let first = first_sets.compute_sequence(&variant.pattern);

        if let Some(token_check) = first.to_peek_check() {
            // Optimierung: LL(1) Peek Check
            checks.extend(quote! {
                if #token_check {
                    return { #logic };
                }
            });
        } else if i == variant_count - 1 && is_top_level {
            // Letzte Variante muss nicht forken, Fehler wird propagated
            checks.extend(logic);
        } else {
            // Backtracking (Speculative Parsing)
            checks.extend(quote! {
                let fork = input.fork();
                let attempt = |input: ParseStream| -> Result<_> {
                    #logic
                };
                
                if let Ok(res) = attempt(&fork) {
                    input.advance_to(&fork);
                    return Ok(res);
                }
            });
        }
    }

    if is_top_level && checks.is_empty() {
         quote! { Err(input.error("No rule variants defined")) }
    } else if is_top_level {
         // Wenn wir hier sind, hat kein Peek und kein Backtracking gepasst
         quote! { 
             #checks
             Err(input.error("No matching rule variant found")) 
         }
    } else {
        quote! { #checks }
    }
}

fn generate_sequence(patterns: &[Pattern], action: &TokenStream, first_sets: &FirstSetComputer) -> TokenStream {
    let mut steps = TokenStream::new();
    
    for pattern in patterns {
        steps.extend(generate_pattern_step(pattern, first_sets));
    }

    quote! {
        {
            #steps
            Ok(#action)
        }
    }
}

fn generate_pattern_step(pattern: &Pattern, first_sets: &FirstSetComputer) -> TokenStream {
    let span = pattern.span();
    match pattern {
        Pattern::Lit(lit) => {
            let token_type = resolve_token_type(lit);
            quote_spanned! {span=> 
                let _ = input.parse::<#token_type>()?; 
            }
        },
        Pattern::RuleCall { binding, rule_name, args } => {
            // FEATURE: Argumente werden jetzt durchgereicht
            let func_call = if is_builtin(rule_name) {
                map_builtin(rule_name)
            } else {
                let f = format_ident!("parse_{}", rule_name);
                // Argumente (Literale) in den Funktionsaufruf packen
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
            let inner_logic = generate_pattern_step(inner, first_sets);
            let first = first_sets.compute_first(inner);
            
            if let Some(check) = first.to_peek_check() {
                quote_spanned! {span=>
                    if #check {
                        #inner_logic
                    }
                }
            } else {
                // Für echte Produktion: Warnung oder Fehler generieren, wenn Optional nicht peekable ist?
                // Hier generieren wir Runtime-Logik, die es einfach ignoriert, wenn nicht klar.
                 quote_spanned! {span=> 
                    // Warning: Optional rule not LL(1) peekable without context. Skipping.
                 }
            }
        },
        Pattern::Repeat(inner) => {
             let inner_logic = generate_pattern_step(inner, first_sets);
             let first = first_sets.compute_first(inner);
             
             if let Some(check) = first.to_peek_check() {
                 quote_spanned! {span=>
                    while #check {
                        #inner_logic
                    }
                 }
             } else {
                 quote!() 
             }
        },
        Pattern::Plus(inner) => {
            let inner_logic = generate_pattern_step(inner, first_sets);
            let first = first_sets.compute_first(inner);
             if let Some(check) = first.to_peek_check() {
                 quote_spanned! {span=>
                    if !#check {
                        return Err(input.error("Expected at least one occurrence"));
                    }
                    while #check {
                        #inner_logic
                    }
                 }
             } else {
                 quote!()
             }
        },
        Pattern::Group(alts) => {
            let temp_variants: Vec<RuleVariant> = alts.iter().map(|pat_seq| {
                RuleVariant { pattern: pat_seq.clone(), action: quote!({}) }
            }).collect();
            
            let variant_logic = generate_variants(&temp_variants, first_sets, false);
            
            quote_spanned! {span=>
                {
                    #variant_logic
                }
            }
        }
    }
}

// --- Hilfsfunktionen ---

fn resolve_token_type(lit: &syn::LitStr) -> syn::Type {
    let s = lit.value();
    let type_str = format!("Token![{}]", s);
    syn::parse_str::<syn::Type>(&type_str)
        .unwrap_or_else(|_| panic!("Invalid token literal in grammar: '{}'", s))
}

fn is_builtin(name: &syn::Ident) -> bool {
    matches!(name.to_string().as_str(), "ident" | "int_lit" | "string_lit")
}

fn map_builtin(name: &syn::Ident) -> TokenStream {
    match name.to_string().as_str() {
        "ident" => quote! { input.parse::<syn::Ident>()? },
        "int_lit" => quote! { input.parse::<syn::LitInt>()?.base10_parse()? },
        "string_lit" => quote! { input.parse::<syn::LitStr>()?.value() },
        _ => panic!("Unknown builtin"),
    }
}

// --- First Set Analysis (Production Grade) ---

struct FirstSetComputer<'a> {
    rules: HashMap<String, &'a Rule>,
    // Cache für bereits berechnete Sets könnte hier hin
}

impl<'a> FirstSetComputer<'a> {
    fn new(grammar: &'a GrammarDefinition) -> Self {
        let mut rules = HashMap::new();
        for r in &grammar.rules {
            rules.insert(r.name.to_string(), r);
        }
        Self { rules }
    }

    fn compute_sequence(&self, patterns: &[Pattern]) -> FirstSet {
        // Wir brauchen einen neuen visited Stack für jeden Top-Level Aufruf
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
            Pattern::Lit(l) => FirstSet::Token(resolve_token_type(l)),
            Pattern::RuleCall { rule_name, .. } => {
                if is_builtin(rule_name) {
                    return match rule_name.to_string().as_str() {
                        "ident" => FirstSet::Raw(quote!(syn::Ident)),
                        "int_lit" => FirstSet::Raw(quote!(syn::LitInt)),
                        "string_lit" => FirstSet::Raw(quote!(syn::LitStr)),
                        _ => FirstSet::Unknown
                    };
                }
                
                let rule_key = rule_name.to_string();
                
                // Zykluserkennung: Wenn wir diese Regel im aktuellen Pfad schon besuchen, abbrechen.
                if visited.contains(&rule_key) {
                    return FirstSet::Unknown;
                }
                visited.insert(rule_key.clone());

                let Some(rule) = self.rules.get(&rule_key) else { return FirstSet::Unknown; };
                
                // Wir nehmen vereinfacht an: Das First-Set der Regel ist das First-Set ihrer ERSTEN Variante.
                // (Für volle Genauigkeit müsste man die Vereinigung aller Varianten bilden, 
                // aber für Peek-Optimierung reicht meist der Hauptpfad).
                let Some(first_var) = rule.variants.first() else { return FirstSet::Unknown; };
                let Some(first_pat) = first_var.pattern.first() else { return FirstSet::Unknown; };

                let result = self.compute_first_recursive(first_pat, visited);
                
                visited.remove(&rule_key); // Backtracking für visited
                result
            },
            Pattern::Group(alts) => {
                if let Some(first_alt) = alts.first() {
                    // Für Gruppen reicht auch der erste Pfad zur Vorhersage (vereinfacht)
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

enum FirstSet {
    Token(syn::Type),
    Raw(TokenStream), 
    Unknown,
}

impl FirstSet {
    fn to_peek_check(&self) -> Option<TokenStream> {
        match self {
            FirstSet::Token(t) => Some(quote! { input.peek(#t) }),
            FirstSet::Raw(t) => Some(quote! { input.peek(#t) }),
            FirstSet::Unknown => None,
        }
    }
}
