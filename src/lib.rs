// src/lib.rs

// 1. Runtime-Module exportieren
// Der vom Makro generierte Code verweist auf `syn_grammar::rt`.
// Daher muss dieses Modul öffentlich verfügbar sein.
pub mod rt;

// 2. Test-Module (optional, nur wenn das Feature 'jit' aktiv ist oder für interne Tests)
#[cfg(feature = "jit")]
pub mod testing;

// 3. Das Makro re-exportieren
// Damit kann der User schreiben: `use syn_grammar::grammar;`
pub use syn_grammar_macros::grammar;
