use std::path::Path;
use proc_macro2::TokenStream;

mod model;      // Dein (generischer) Meta-AST
mod parser;     // Parst die .grammar Dateien
mod resolver;   // Löst Imports/Vererbung auf (unverändert)
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

    // Test 1: Parser Test für EBNF Features
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
        
        // Check: 3 Patterns (start, group*, end?)
        assert_eq!(variant.pattern.len(), 3);
        
        // Prüfen, ob Pattern 2 ein Repeat ist
        match &variant.pattern[1] {
            Pattern::Repeat(inner) => {
                match &**inner {
                    Pattern::Group(alts) => assert_eq!(alts.len(), 2), // "a" | "b"
                    _ => panic!("Expected Group inside Repeat"),
                }
            },
            _ => panic!("Expected Repeat pattern"),
        }
    }

    // Test 2: Codegen Test - Prüfen auf Control Flow und Forking
    #[test]
    fn test_codegen_structures() {
        let input = r#"
            grammar Logic {
                // Das hier erfordert Backtracking (Fork), da beide mit "ident" anfangen könnten
                // wenn wir nicht genau hinschauen (oder für den Testfall erzwingen wir es)
                rule ambiguous -> i32 = 
                    a:ident() "x" -> { 1 }
                  | b:ident() "y" -> { 2 }
            }
        "#;
        
        let grammar: GrammarDefinition = syn::parse_str(input).unwrap();
        let rust_code = codegen::generate_rust(grammar);
        let code_str = rust_code.to_string();

        // 1. Wir erwarten, dass input.fork() generiert wird
        assert!(code_str.contains("input . fork ()"), "Should generate forking code");
        
        // 2. Wir erwarten Closures für die Versuche
        assert!(code_str.contains("| input : ParseStream |"), "Should generate closure for backtracking");
    }
    
    // Test 3: Integration Snapshot
    #[test]
    fn test_full_pipeline() {
        let input = r#"
            grammar Calc {
                rule expr -> i32 = 
                    t:term() ( "+" t2:term() )* -> { 0 } // Vereinfachte Action

                rule term -> i32 = 
                    f:factor() ( "*" f2:factor() )* -> { 0 }

                rule factor -> i32 = 
                    "(" e:expr() ")" -> { e }
                  | i:int_lit() -> { i }
            }
        "#;
        
        let grammar: GrammarDefinition = syn::parse_str(input).unwrap();
        let rust_code = codegen::generate_rust(grammar);
        
        // Mit 'insta' prüfen wir, ob der Output stabil bleibt
        // (Insta muss in Cargo.toml dev-dependencies sein, sonst auskommentieren)
        // insta::assert_display_snapshot!(rust_code);
        
        // Fallback Assert, falls kein insta:
        let s = rust_code.to_string();
        assert!(s.contains("while input . peek (Token ! [ + ])"));
        assert!(s.contains("fn parse_expr"));
    }
}
