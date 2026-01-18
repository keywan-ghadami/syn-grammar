use crate::model::GrammarDefinition;
use std::path::{Path, PathBuf};
use std::fs;

pub struct GrammarResolver {
    base_dir: PathBuf,
}

impl GrammarResolver {
    pub fn new<P: AsRef<Path>>(base_dir: P) -> Self {
        Self {
            base_dir: base_dir.as_ref().to_path_buf(),
        }
    }

    pub fn resolve(&self, filename: &str) -> Result<GrammarDefinition, Box<dyn std::error::Error>> {
        // 1. Pfad bauen
        let file_path = self.base_dir.join(filename);
        
        // 2. Datei lesen
        let content = fs::read_to_string(&file_path)
            .map_err(|e| format!("Failed to read grammar file {:?}: {}", file_path, e))?;

        // 3. Parsen (nutzt unseren parser.rs via syn)
        let grammar: GrammarDefinition = syn::parse_str(&content)?;

        // HINWEIS: Hier würde später die Vererbungslogik (Inheritance) stattfinden.
        // Wenn grammar.inherits gesetzt ist, würde man rekursiv resolve() aufrufen 
        // und die Regeln mergen. Für Stage 0 reicht das Laden der einzelnen Datei.

        Ok(grammar)
    }
}

