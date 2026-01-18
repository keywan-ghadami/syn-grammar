use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use syn_grammar::Generator;

pub struct TestEnv {
    _temp_dir: tempfile::TempDir,
    #[allow(dead_code)] // Wird aktuell nur gehalten, damit das TempDir existiert
    project_path: PathBuf,
    binary_path: PathBuf,
}

impl TestEnv {
    pub fn new(grammar_name: &str, grammar_content: &str) -> Self {
        let temp_dir = tempfile::tempdir().expect("Could not create temp dir");
        let project_path = temp_dir.path().to_path_buf();
        
        setup_cargo_project(&project_path, grammar_name);

        let generator = Generator::new(&project_path);
        
        let grammar_file_path = project_path.join(format!("{}.grammar", grammar_name));
        fs::write(&grammar_file_path, grammar_content).expect("Failed to write grammar file");

        let rust_code = generator.generate(&format!("{}.grammar", grammar_name))
            .expect("Code generation failed");

        // Wrapper Code
        let main_rs = format!(r#"
            use syn::parse::Parser; 
            use proc_macro2::TokenStream;
            use quote::{{quote, ToTokens}};
            use std::io::Read;

            // --- GENERATED CODE START ---
            {}
            // --- GENERATED CODE END ---

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

        let mut cmd = Command::new("cargo");
        cmd.arg("build").current_dir(&project_path);

        let output = cmd.output().expect("Failed to execute cargo build");
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            panic!("Compilation failed:\n{}", stderr);
        }

        let binary_path = project_path.join("target/debug/test_parser");
        Self { _temp_dir: temp_dir, project_path, binary_path }
    }

    pub fn parse(&self, input: &str) -> (String, String, bool) {
        let mut child = Command::new(&self.binary_path)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("Failed to spawn parser process");

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

    let cargo_toml = format!(r#"
[package]
name = "test_parser"
version = "0.0.1"
edition = "2024"

[dependencies]
syn = {{ version = "2.0", features = ["full", "parsing", "printing", "extra-traits"] }}
quote = "1.0"
proc-macro2 = "1.0"
    "#);

    fs::write(path.join("Cargo.toml"), cargo_toml).unwrap();
}
