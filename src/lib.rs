use std::path::Path;
use proc_macro2::TokenStream;

mod model;      // Der AST
mod parser;     // Parst .grammar Dateien
mod resolver;   // Löst Imports auf
mod codegen;    // Der Rust-Code Generator

// Unsere neue Runtime-Library muss public sein
pub mod rt;

pub struct Generator {
    resolver: resolver::GrammarResolver,
}

impl Generator {
    pub fn new<P: AsRef<Path>>(base_dir: P) -> Self {
        Self {
            resolver: resolver::GrammarResolver::new(base_dir),
        }
    }

    pub fn generate(&self, root_file: &str) -> Result<TokenStream, Box<dyn std::error::Error>> {
        let grammar_def = self.resolver.resolve(root_file)?;
        let code = codegen::generate_rust(grammar_def);
        Ok(code)
    }
}

// --- UNIT TESTS ---
#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;

    // Hilfsfunktion: Entfernt alle Whitespaces für robusten String-Vergleich
    fn normalize(s: &str) -> String {
        s.chars().filter(|c| !c.is_whitespace()).collect()
    }

    #[test]
    fn test_parser_ebnf() {
        // Testet, ob unsere Grammatik-Syntax korrekt in den AST geparst wird
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
        
        // start, group*, end? -> 3 Patterns
        assert_eq!(variant.pattern.len(), 3);
        
        // Prüfen, ob das zweite Element ein Repeat(Group(...)) ist
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

    #[test]
    fn test_codegen_structures() {
        // Testet, ob der Generator korrekten Code für Ambiguitäten erzeugt
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

        // UPDATE: Da wir jetzt 'rt::parse_speculative' nutzen, suchen wir danach
        // statt nach rohem 'input.fork()'.
        assert!(code_str.contains("rt::parse_speculative"), "Should generate call to runtime speculative parsing");
        
        // Wir prüfen auch, ob die Closure generiert wird
        assert!(code_str.contains("|input|"), "Should generate closure for backtracking");
    }
    
    #[test]
    fn test_full_pipeline() {
        // Testet eine komplexere Grammatik auf generierte Token-Checks
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
        
        // Repeat erzeugt eine while Schleife mit Peek Check
        assert!(code_str.contains("whileinput.peek(Token![+])"), "Repeat-Schleife mit '+' Token fehlt");
        assert!(code_str.contains("whileinput.peek(Token![*])"), "Repeat-Schleife mit '*' Token fehlt");
        
        // Funktionsname muss generiert sein
        assert!(code_str.contains("fnparse_expr(input:ParseStream)"), "Funktionssignatur parse_expr fehlt");
    }
}
