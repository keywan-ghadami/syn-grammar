#![cfg(feature = "jit")]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use crate::Generator;

/// Kapselt das Ergebnis eines Test-Laufs
pub struct TestResult {
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
}

impl TestResult {
    /// Behauptet, dass der Test erfolgreich war. Falls nicht, wird ein detaillierter
    /// Panic mit Stdout/Stderr ausgelöst.
    pub fn assert_success(&self) {
        if !self.success {
            panic!(
                "\n🔴 TEST FAILED:\n\n=== STDERR ===\n{}\n\n=== STDOUT ===\n{}\n",
                self.stderr, self.stdout
            );
        }
    }

    /// Behauptet, dass der Test fehlgeschlagen ist (z.B. für negative Tests).
    pub fn assert_failure(&self) {
        if self.success {
            panic!("\n🔴 TEST UNEXPECTEDLY SUCCEEDED (Expected Failure)\n");
        }
    }
    
    /// Prüft, ob der Stdout einen bestimmten String enthält
    pub fn contains(&self, needle: &str) -> bool {
        self.stdout.contains(needle)
    }
}

pub struct TestEnv {
    _temp_dir: tempfile::TempDir,
    project_path: PathBuf,
}

impl TestEnv {
    pub fn new(grammar_name: &str, grammar_content: &str) -> Self {
        let temp_dir = tempfile::tempdir().expect("Could not create temp dir");
        let project_path = temp_dir.path().to_path_buf();
        
        setup_cargo_project(&project_path, grammar_name);

        let generator = Generator::new(&project_path);
        
        let grammar_file_path = project_path.join(format!("{}.grammar", grammar_name));
        fs::write(&grammar_file_path, grammar_content).expect("Failed to write grammar file");

        // Hier fangen wir Errors beim Generieren ab, damit der Test aussagekräftig failt
        let rust_code = generator.generate(&format!("{}.grammar", grammar_name))
            .expect("❌ Code generation failed inside Generator::generate");

        let main_rs = format!(r#"
            #![allow(unused_imports, dead_code, unused_variables)]
            use syn::parse::Parser; 
            use proc_macro2::TokenStream;
            use std::io::Read;

            mod generated {{
                {}
            }}
            use generated::parse_main; // Annahme: Einstiegspunkt ist 'main'

            fn main() {{
                let mut content = String::new();
                std::io::stdin().read_to_string(&mut content).unwrap();

                let stream: TokenStream = match syn::parse_str(&content) {{
                    Ok(ts) => ts,
                    Err(e) => {{
                         eprintln!("Tokenization Error: {{}}", e);
                         std::process::exit(1);
                    }}
                }};

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

    /// Gibt nun ein TestResult zurück statt einem Tuple
    pub fn parse(&self, input: &str) -> TestResult {
        let mut cmd = Command::new("cargo");
        cmd.arg("run")
           .arg("--quiet")
           .current_dir(&self.project_path)
           .stdin(std::process::Stdio::piped())
           .stdout(std::process::Stdio::piped())
           .stderr(std::process::Stdio::piped());

        let mut child = cmd.spawn().expect("Failed to spawn cargo run");

        {
            use std::io::Write;
            let stdin = child.stdin.as_mut().expect("Failed to open stdin");
            stdin.write_all(input.as_bytes()).expect("Failed to write to stdin");
        }

        let output = child.wait_with_output().expect("Failed to read output");
        
        TestResult {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            success: output.status.success(),
        }
    }
}

fn setup_cargo_project(path: &Path, _name: &str) {
    let src_dir = path.join("src");
    fs::create_dir_all(&src_dir).unwrap();

    let current_dir = std::env::current_dir().unwrap();
    let current_dir_str = current_dir.to_string_lossy();

    // Verwende absoluten Pfad und deaktiviere JIT im Child-Projekt, um Rekursion zu vermeiden
    let cargo_toml = format!(r#"
[package]
name = "jit_parser"
version = "0.0.1"
edition = "2024"

[dependencies]
syn = {{ version = "2.0", features = ["full", "parsing", "printing", "extra-traits"] }}
quote = "1.0"
proc-macro2 = "1.0"
syn-grammar = {{ path = "{}", default-features = false }} 
    "#, current_dir_str);

    fs::write(path.join("Cargo.toml"), cargo_toml).unwrap();
}
