// src/lib.rs

// 1. Runtime-Module exportieren
// Der vom Makro generierte Code verweist auf `syn_grammar::rt`.
// Daher muss dieses Modul öffentlich verfügbar sein.
pub mod rt;
pub mod testing;

// 3. Das Makro re-exportieren
// Damit kann der User schreiben: `use syn_grammar::grammar;`
pub use syn_grammar_macros::grammar;
