// syn-grammar/src/lib.rs
use std::path::Path;
use proc_macro2::TokenStream;

mod model;      // Dein (generischer) Meta-AST
mod parser;     // Parst die .grammar Dateien (in syn)
mod resolver;   // Löst Imports/Vererbung auf
mod codegen;    // Der Generator

pub struct Generator {
    resolver: resolver::GrammarResolver,
}

impl Generator {
    pub fn new<P: AsRef<Path>>(base_dir: P) -> Self {
        Self {
            resolver: resolver::GrammarResolver::new(base_dir),
        }
    }

    /// Der Hauptaufruf: Wandelt eine Grammatik-Datei in Rust-Code um.
    pub fn generate(&self, root_file: &str) -> Result<TokenStream, Box<dyn std::error::Error>> {
        // 1. Auflösen (Imports, Vererbung)
        let grammar_def = self.resolver.resolve(root_file)?;
        
        // 2. Generieren
        let code = codegen::generate_rust(grammar_def);
        
        Ok(code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;
    use crate::model::*;

    // Test 1: Parser Test (Liest der Parser die Grammatik korrekt?)
    #[test]
    fn test_parser_simple() {
        let input = r#"
            grammar Test {
                pub rule main -> i32 = "val" v:int_lit() -> { v }
            }
        "#;
        
        let grammar: GrammarDefinition = syn::parse_str(input).expect("Parsing failed");
        assert_eq!(grammar.name.to_string(), "Test");
        assert_eq!(grammar.rules.len(), 1);
    }

    // Test 2: Codegen Test (Erzeugt er validen Rust Code?)
    #[test]
    fn test_codegen_output() {
        // Wir bauen manuell einen kleinen AST, um den Parser zu umgehen
        let grammar = GrammarDefinition {
            name: format_ident!("Test"),
            inherits: None,
            rules: vec![
                Rule {
                    is_pub: true,
                    name: format_ident!("start"),
                    return_type: parse_quote!(i32),
                    variants: vec![
                        RuleVariant {
                            pattern: vec![
                                Pattern::Lit(syn::parse_str("\"add\"").unwrap())
                            ],
                            action: quote! { 0 }
                        }
                    ]
                }
            ]
        };

        let output = codegen::generate_rust(grammar);
        let output_str = output.to_string();

        // Prüfen ob der Output das enthält, was wir erwarten
        assert!(output_str.contains("pub fn parse_start"));
        assert!(output_str.contains("Token ! [ add ]"));
    }
    
    // Test 3: Integration (String rein -> Rust Code raus)
    // Das simuliert den echten Ablauf in build.rs
    #[test]
    fn test_full_pipeline() {
        let input = r#"
            grammar Calc {
                rule add -> i32 = "plus" -> { 1 }
            }
        "#;
        
        let grammar: GrammarDefinition = syn::parse_str(input).unwrap();
        let rust_code = codegen::generate_rust(grammar);
        
        // Mit 'insta' prüfen wir, ob sich der generierte Code ungewollt ändert
        insta::assert_display_snapshot!(rust_code);
    }
}
