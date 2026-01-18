use std::path::Path;
use proc_macro2::TokenStream;

mod model;
mod parser;
mod resolver;
mod codegen;

pub mod rt;

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
        // 1. Auflösen
        let grammar_ast = self.resolver.resolve(root_file)?;
        
        // 2. Konvertieren (Parser -> Model)
        let model: model::GrammarDefinition = grammar_ast.into();
        
        // 3. Generieren (mit Result Fehlerbehandlung)
        // proc_macro_error::abort! würde hier panic-en, wenn wir in einem Makro wären.
        // Da wir in einer lib sind, fangen wir Panics idealerweise ab oder nutzen Result.
        // Aktuell gibt codegen::generate_rust ein Result zurück.
        let code = codegen::generate_rust(model)?;
        
        Ok(code)
    }
}

// Unit Tests
#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;

    fn normalize(s: &str) -> String {
        s.chars().filter(|c| !c.is_whitespace()).collect()
    }

    #[test]
    fn test_codegen_uses_combinators() {
        let input = r#"
            grammar Test {
                rule a -> () = "start" "end" -> { () }
            }
        "#;
        
        let p_ast: parser::GrammarDefinition = syn::parse_str(input).unwrap();
        let model: GrammarDefinition = p_ast.into();
        let code = codegen::generate_rust(model).unwrap();
        let s = normalize(&code.to_string());
        
        // Wir erwarten Peek Checks für Literale
        assert!(s.contains("input.peek(Token![start])"));
    }
}
