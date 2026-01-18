use std::path::Path;
use proc_macro2::TokenStream;

mod model;      // Dein (generischer) Meta-AST
mod parser;     // Parst die .grammar Dateien
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
    use crate::model::*;

    // Hilfsfunktion: Entfernt alle Whitespaces für robusten Vergleich
    fn normalize(s: &str) -> String {
        s.chars().filter(|c| !c.is_whitespace()).collect()
    }

    #[test]
    fn test_parser_ebnf() {
        let input = r#"
            grammar EbnfTest {
                rule test -> () = 
                    "start" 
                    ( "a" | "b" )* "end"? 
                    -> { () }
            }
        "#;
        
        let grammar: GrammarDefinition = syn::parse_str(input).expect("Parsing failed");
        let rule = &grammar.rules[0];
        let variant = &rule.variants[0];
        
        assert_eq!(variant.pattern.len(), 3);
        
        match &variant.pattern[1] {
            Pattern::Repeat(inner) => {
                match &**inner {
                    Pattern::Group(alts) => assert_eq!(alts.len(), 2),
                    _ => panic!("Expected Group inside Repeat"),
                }
            },
            _ => panic!("Expected Repeat pattern"),
        }
    }

    #[test]
    fn test_codegen_structures() {
        let input = r#"
            grammar Logic {
                rule ambiguous -> i32 = 
                    a:ident() "x" -> { 1 }
                  | b:ident() "y" -> { 2 }
            }
        "#;
        
        let grammar: GrammarDefinition = syn::parse_str(input).unwrap();
        let rust_code = codegen::generate_rust(grammar);
        let code_str = normalize(&rust_code.to_string());

        // Wir prüfen auf normalisierte Strings (ohne Leerzeichen)
        assert!(code_str.contains("input.fork()"), "Should generate forking code");
        assert!(code_str.contains("|input:ParseStream|"), "Should generate closure");
    }
    
    #[test]
    fn test_full_pipeline() {
        let input = r#"
            grammar Calc {
                rule expr -> i32 = 
                    t:term() ( "+" t2:term() )* -> { 0 }

                rule term -> i32 = 
                    f:factor() ( "*" f2:factor() )* -> { 0 }

                rule factor -> i32 = 
                    "open" e:expr() "close" -> { e }
                  | i:int_lit() -> { i }
            }
        "#;
        
        let grammar: GrammarDefinition = syn::parse_str(input).unwrap();
        let rust_code = codegen::generate_rust(grammar);
        let code_str = normalize(&rust_code.to_string());
        
        // Prüfen auf wesentliche Bestandteile (Whitespace-agnostisch)
        assert!(code_str.contains("whileinput.peek(Token![+])"), "Repeat-Schleife mit '+' Token fehlt");
        assert!(code_str.contains("whileinput.peek(Token![*])"), "Repeat-Schleife mit '*' Token fehlt");
        assert!(code_str.contains("fnparse_expr(input:ParseStream)"), "Funktionssignatur parse_expr fehlt");
    }
}
