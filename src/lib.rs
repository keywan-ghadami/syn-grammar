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
        // Resolver gibt Parser-Output zurück
        let grammar_ast = self.resolver.resolve(root_file)?;
        
        // Konvertieren ins saubere Modell
        let model: model::GrammarDefinition = grammar_ast.into();
        
        let code = codegen::generate_rust(model)?;
        Ok(code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;

    fn normalize(s: &str) -> String {
        s.chars().filter(|c| !c.is_whitespace()).collect()
    }

    #[test]
    fn test_parser_conversion() {
        let input = r#"
            grammar Test {
                pub rule main -> () = "x" -> { () }
            }
        "#;
        let p_ast: parser::GrammarDefinition = syn::parse_str(input).unwrap();
        let m_ast: GrammarDefinition = p_ast.into();
        
        assert_eq!(m_ast.rules.len(), 1);
        assert!(m_ast.rules[0].is_pub);
    }
}
