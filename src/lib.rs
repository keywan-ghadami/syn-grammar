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
    // Unused import 'syn::parse_quote' removed
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
                // Das hier erfordert Backtracking (Fork), da beide mit "ident" anfangen
                rule ambiguous -> i32 = 
                    a:ident() "x" -> { 1 }
                  | b:ident() "y" -> { 2 }
            }
        "#;
        
        let grammar: GrammarDefinition = syn::parse_str(input).unwrap();
        let rust_code = codegen::generate_rust(grammar);
        let code_str = rust_code.to_string();

        // Debug output falls der Test failed
        // println!("{}", code_str);

        // 1. Wir erwarten, dass input.fork() generiert wird
        // Da 'ident' bei beiden Regeln der Start ist, MUSS geforkt werden.
        assert!(code_str.contains("input . fork ()"), "Should generate forking code because of ambiguity");
        
        // 2. Wir erwarten Closures für die Versuche
        assert!(code_str.contains("| input : ParseStream |"), "Should generate closure for backtracking");
    }
    
    // Test 3: Integration Snapshot
    #[test]
    fn test_full_pipeline() {
        // HINWEIS: Wir vermeiden hier "(" und ")", da syn diese nicht als einzelne 
        // Tokens parsen kann (Token![paren] gibt es nicht für Parsing).
        // Stattdessen nutzen wir "open" und "close" dummies oder Operatoren.
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
        
        let s = rust_code.to_string();
        assert!(s.contains("while input . peek (Token ! [ + ])"));
        assert!(s.contains("fn parse_expr"));
        // Prüfen ob literals korrekt aufgelöst wurden
        assert!(s.contains("Token ! [ * ]"));
    }
}
