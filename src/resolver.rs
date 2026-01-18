use std::path::{Path, PathBuf};
use std::fs;
use crate::parser;
use syn::parse_str;

pub struct GrammarResolver {
    base_dir: PathBuf,
}

impl GrammarResolver {
    pub fn new<P: AsRef<Path>>(base_dir: P) -> Self {
        Self {
            base_dir: base_dir.as_ref().to_path_buf(),
        }
    }

    pub fn resolve(&self, filename: &str) -> Result<parser::GrammarDefinition, Box<dyn std::error::Error>> {
        let path = self.base_dir.join(filename);
        let content = fs::read_to_string(&path)?;
        
        let grammar: parser::GrammarDefinition = parse_str(&content)?;
        
        // TODO: Hier könnte man Imports rekursiv auflösen (future work)
        
        Ok(grammar)
    }
}
