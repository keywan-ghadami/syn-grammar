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

