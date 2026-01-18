#![cfg(feature = "jit")]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use crate::Generator;

/// Eine Umgebung, die eine Grammatik on-the-fly kompiliert und ausführbar macht.
pub struct TestEnv {
    _temp_dir: tempfile::TempDir,
    project_path: PathBuf,
}

impl TestEnv {
    pub fn new(grammar_name: &str, grammar_content: &str) -> Self {
        let temp_dir = tempfile::tempdir().expect("Could not create temp dir");
        let project_path = temp_dir.path().to_path_buf();
        
        // 1. Cargo Projekt aufsetzen
        setup_cargo_project(&project_path, grammar_name);

        // 2. Code generieren
        let generator = Generator::new(&project_path);
        
        let grammar_file_path = project_path.join(format!("{}.grammar", grammar_name));
        fs::write(&grammar_file_path, grammar_content).expect("Failed to write grammar file");

        let rust_code = generator.generate(&format!("{}.grammar", grammar_name))
            .expect("Code generation failed");

        // 3. Wrapper generieren (main.rs)
        let main_rs = format!(r#"
            #![allow(unused_imports, dead_code, unused_variables)]

            use syn::parse::Parser; 
            use proc_macro2::TokenStream;
            use std::io::Read;

            // Das generierte Modul
            mod generated {{
                {}
            }}

            use generated::parse_main;

            fn main() {{
                let mut content = String::new();
                std::io::stdin().read_to_string(&mut content).unwrap();

                // 1. Tokenisierung (Syn)
                let stream: TokenStream = match syn::parse_str(&content) {{
                    Ok(ts) => ts,
                    Err(e) => {{
                         eprintln!("Tokenization Error: {{}}", e);
                         std::process::exit(1);
                    }}
                }};

                // 2. Parsing (Unser generierter Parser)
                match parse_main.parse2(stream) {{
                    Ok(ast) => println!("{{:?}}", ast),
                    Err(e) => {{
                        eprintln!("Parse Error: {{}}", e);
                        std::process::exit(1);
                    }}
                }}
            }}
        "#, rust_code);

        fs::write(project_path.join("src/main.rs"), main_rs).expect("Failed to write main.rs");

        Self { _temp_dir: temp_dir, project_path }
    }

    /// Führt den Parser mit einem Input aus und gibt (Stdout, Stderr, Success) zurück
    pub fn parse(&self, input: &str) -> (String, String, bool) {
        // Wir nutzen 'cargo run', das kompiliert (inkrementell) und führt aus.
        let mut cmd = Command::new("cargo");
        cmd.arg("run")
           .arg("--quiet") // Weniger Lärm im Output
           .current_dir(&self.project_path)
           .stdin(std::process::Stdio::piped())
           .stdout(std::process::Stdio::piped())
           .stderr(std::process::Stdio::piped());

        // Optional: Cranelift für Speed (wenn installiert)
        // cmd.env("RUSTFLAGS", "-Zcodegen-backend=cranelift");

        let mut child = cmd.spawn().expect("Failed to spawn cargo run");

        {
            use std::io::Write;
            let stdin = child.stdin.as_mut().expect("Failed to open stdin");
            stdin.write_all(input.as_bytes()).expect("Failed to write to stdin");
        }

        let output = child.wait_with_output().expect("Failed to read output");
        
        (
            String::from_utf8_lossy(&output.stdout).to_string(),
            String::from_utf8_lossy(&output.stderr).to_string(),
            output.status.success()
        )
    }
}

fn setup_cargo_project(path: &Path, _name: &str) {
    let src_dir = path.join("src");
    fs::create_dir_all(&src_dir).unwrap();

    // Pfad zum aktuellen syn-grammar (dieses Projekt) ermitteln
    let current_dir = std::env::current_dir().unwrap();
    let current_dir_str = current_dir.to_string_lossy();

    // Wir erstellen eine Cargo.toml für das temporäre Projekt.
    // Wichtig: Wir binden 'syn-grammar' via Pfad ein, damit wir auf 'rt' zugreifen können.
    let cargo_toml = format!(r#"
[package]
name = "jit_parser"
version = "0.0.1"
edition = "2024"

[dependencies]
syn = {{ version = "2.0", features = ["full", "parsing", "printing", "extra-traits"] }}
quote = "1.0"
proc-macro2 = "1.0"
syn-grammar = {{ path = "{}" }} 
    "#, current_dir_str);

    fs::write(path.join("Cargo.toml"), cargo_toml).unwrap();
}
