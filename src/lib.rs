use std::path::Path;
use proc_macro2::TokenStream;

mod model;      // Der AST
mod parser;     // Parst .grammar Dateien
mod resolver;   // Löst Imports auf
mod codegen;    // Der Rust-Code Generator

// Runtime Library (Muss public sein)
pub mod rt;

// Testing / JIT Infrastructure (Nur aktiv mit Feature "jit")
#[cfg(feature = "jit")]
pub mod testing;

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
// Diese Tests laufen schnell und prüfen die interne Logik, ohne Cargo zu starten.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;

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
        
        // Prüfen der verschachtelten Struktur
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
    fn test_codegen_calls_rt() {
        // Prüft, ob der Generator die neue Runtime-Library nutzt
        let input = r#"
            grammar Logic {
                rule ambig -> i32 = 
                    a:ident() "x" -> { 1 }
                  | b:ident() "y" -> { 2 }
            }
        "#;
        
        let grammar: GrammarDefinition = syn::parse_str(input).unwrap();
        let rust_code = codegen::generate_rust(grammar);
        let code_str = normalize(&rust_code.to_string());

        // Wir erwarten rt::parse_speculative statt input.fork()
        assert!(code_str.contains("rt::parse_speculative"));
        // Wir erwarten rt::parse_ident statt syn::Ident::parse_any
        assert!(code_str.contains("rt::parse_ident"));
    }

    #[test]
    fn test_generated_structure() {
        // Prüft, ob Funktionssignaturen korrekt generiert werden
        let input = r#"
            grammar Calc {
                pub rule main -> i32 = "x" -> { 0 }
                rule sub -> () = "y" -> { () }
            }
        "#;
        
        let grammar: GrammarDefinition = syn::parse_str(input).unwrap();
        let rust_code = codegen::generate_rust(grammar);
        let s = normalize(&rust_code.to_string());

        // 1. Prüfen auf Public Function
        assert!(s.contains("pubfnparse_main"), "Missing pub fn parse_main");
        
        // 2. Prüfen auf Private Function
        // (Beachte: normalize entfernt Leerzeichen, daher 'fnparse_sub')
        assert!(s.contains("fnparse_sub"), "Missing fn parse_sub");
        
        // 3. Prüfen auf Grammar Name Konstante
        assert!(s.contains("pubconstGRAMMAR_NAME:&str=stringify!(Calc)"));
    }
}
