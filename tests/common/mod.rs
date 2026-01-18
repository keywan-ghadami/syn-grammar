use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use syn_grammar::Generator;

pub struct TestEnv {
    _temp_dir: tempfile::TempDir,
    project_path: PathBuf,
    binary_path: PathBuf,
}

impl TestEnv {
    /// Erstellt eine neue Test-Umgebung, generiert den Parser-Code und kompiliert ihn.
    pub fn new(grammar_name: &str, grammar_content: &str) -> Self {
        let temp_dir = tempfile::tempdir().expect("Could not create temp dir");
        let project_path = temp_dir.path().to_path_buf();
        
        // 1. Setup Cargo Project Structure
        setup_cargo_project(&project_path, grammar_name);

        // 2. Generate Parser Code
        let generator = Generator::new(&project_path);
        
        // Wir schreiben die Grammar-Datei temporär, damit der Resolver sie findet
        let grammar_file_path = project_path.join(format!("{}.grammar", grammar_name));
        fs::write(&grammar_file_path, grammar_content).expect("Failed to write grammar file");

        // Generieren
        let rust_code = generator.generate(&format!("{}.grammar", grammar_name))
            .expect("Code generation failed");

        // 3. Create Main Wrapper
        // Wir wickeln den generierten Code in eine main-Funktion, die von STDIN liest
        // und das Ergebnis via Debug-Output ausgibt.
        let full_source = format!(r#"
            // Dependencies (simuliert via Cargo.toml unten)
            use std::io::Read;
            use syn::parse::Parse;
            
            // --- GENERATED CODE START ---
            {}
            // --- GENERATED CODE END ---

            fn main() {{
                let mut input = String::new();
                std::io::stdin().read_to_string(&mut input).unwrap();
                
                // Wir nutzen syn::parse_str um den generierten Parser aufzurufen
                // Da wir den Namen der Start-Regel nicht kennen, nehmen wir an, der Test nutzt die erste Regel
                // oder wir parsen explizit.
                // Um es generisch zu machen, generiert der Generator Code. 
                // Hier in Stage 1 müssen wir wissen, wie die Funktion heißt.
                // Wir suchen die 'fn parse_start' Konvention im Test-Setup oder wir machen es via macro hack?
                
                // HACK: Für die Tests nehmen wir an, dass die Hauptregel "root" heißt.
                match syn::parse_str::<Root>(&input) {{
                    Ok(res) => println!("{{:?}}", res),
                    Err(e) => eprintln!("Parse Error: {{}}", e),
                }}
            }}

            // Helper struct to verify output
            #[derive(Debug)]
            struct Root(pub i32); // Placeholder, wird vom generierten Code überschrieben hoffentlich?
            // NEIN, wir können den Typ nicht einfach überschreiben.
            // Lösung: Der TestEnv muss wissen, was der Return-Type ist, oder wir generieren Code,
            // der 'impl Parse for ...' nutzt.
            
            // BESSERER ANSATZ für den Wrapper:
            // Wir parsen den Output im generierten Code nicht direkt in main, 
            // sondern wir erwarten, dass der generierte Code eigenständige Typen hat.
            // Da unser aktueller Generator nur Funktionen `parse_X` generiert und `syn` Typen zurückgibt,
            // müssen wir den generierten Code so nutzen:
            
            fn run_parser(input: &str) -> syn::Result<String> {{
                // Wir hardcoden hier den Aufruf der 'parse_main' Funktion.
                // Das bedeutet, deine Test-Grammatik MUSS eine Regel namens 'main' haben.
                let stream: proc_macro2::TokenStream = input.parse()?;
                let result = parse_main.parse2(stream)?; // parse_main wird generiert
                Ok(format!("{{:?}}", result))
            }}
        "#, rust_code);
        
        // KORREKTUR: Der Generator erzeugt `fn parse_NAME`. Wir brauchen eine `main`, die das aufruft.
        // Da wir den Return-Type der Regel nicht kennen (i32, (), etc.), nutzen wir `Debug` Output.
        // Wir schreiben eine spezialisierte `main.rs`, die direkt auf die generierte Funktion zugreift.
        
        let main_rs = format!(r#"
            use syn::{{parse::Parse, parse::ParseStream, Result, Token}};
            use quote::{{quote, ToTokens}}; // Falls Actions TokenStreams zurückgeben

            // Generierter Code
            {}

            fn main() {{
                let mut content = String::new();
                std::io::stdin().read_to_string(&mut content).unwrap();

                // Wir parsen den Input string in einen TokenStream
                // HINWEIS: syn::parse_str erwartet Rust-Tokens. 
                // Wenn deine Grammatik "custom syntax" parst, ist das okay, solange es tokenizable ist.
                
                // Wir rufen die generierte Funktion `parse_main` auf.
                // Voraussetzung: Die Grammatik hat eine Regel `rule main`.
                let result = match syn::parse_str::<syn::export::TokenStream>(&content) {{
                    Ok(ts) => parse_main.parse2(ts),
                    Err(e) => {{
                         eprintln!("Tokenization Error: {{}}", e);
                         std::process::exit(1);
                    }}
                }};

                match result {{
                    Ok(ast) => println!("{{:?}}", ast),
                    Err(e) => {{
                        eprintln!("Parse Error: {{}}", e);
                        std::process::exit(1);
                    }}
                }}
            }}
        "#, rust_code);

        fs::write(project_path.join("src/main.rs"), main_rs).expect("Failed to write main.rs");

        // 4. Compile
        // Wir nutzen 'cargo build', um alle Dependencies zu holen.
        let mut cmd = Command::new("cargo");
        cmd.arg("build").current_dir(&project_path);
        
        // Optional: Cranelift Backend aktivieren (falls installiert)
        // cmd.env("RUSTFLAGS", "-Zcodegen-backend=cranelift"); 
        // (Auskommentiert, da dies Nightly + Component benötigt. Für CI Standard lassen.)

        let output = cmd.output().expect("Failed to execute cargo build");
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            panic!("Compilation failed:\n{}", stderr);
        }

        let binary_path = project_path.join("target/debug/test_parser");
        Self { _temp_dir: temp_dir, project_path, binary_path }
    }

    /// Führt den Parser mit einem Input aus und gibt (Stdout, Stderr, Success) zurück
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

fn setup_cargo_project(path: &Path, name: &str) {
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
