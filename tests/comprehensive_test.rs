mod common;
use common::TestEnv;

#[test]
fn test_complex_grammar_features() {
    // 1. Definiere eine mächtige Grammatik, die alles testet
    // Wir bauen einen kleinen Lisp-ähnlichen Parser + mathematische Ausdrücke
    let grammar = r#"
        grammar Comprehensive {
            // Einstiegspunkt (Muss 'main' heißen für unseren Test-Runner)
            rule main -> String = 
                  e:expr() -> { format!("Expr: {}", e) }
                | l:list() -> { format!("List: {}", l) }

            // Rekursive Ausdrücke mit Precedence (simuliert durch Hierarchie)
            rule expr -> String = 
                t:term() ( "+" t2:term() )* -> { 
                    // Wir geben hier Strings zurück, um das Ergebnis leicht zu prüfen
                    "Add(...)".to_string() 
                }

            rule term -> String = 
                f:factor() ( "*" f2:factor() )* -> { "Mul(...)".to_string() }

            rule factor -> String = 
                  i:int_lit() -> { i.to_string() }
                | "(" e:expr() ")" -> { format!("({})", e) }
                | "nop" -> { "Nop".to_string() }

            // Listen mit Wiederholungen und Optionen
            rule list -> String = 
                "[" 
                head:ident() 
                ( "," tail:ident() )* trailing:","? 
                "]" 
                -> { 
                    format!("List[head={}, tail_count=...]", head) 
                }
            
            // Test für Gruppen (Nested Patterns)
            rule grouped -> i32 =
                ( "a" | "b" ) "c" -> { 0 }
        }
    "#;

    // 2. Compile (One-time cost)
    // Das dauert ein paar Sekunden, aber nur einmal für alle Assertions unten.
    let env = TestEnv::new("Comprehensive", grammar);

    // 3. Run Tests (Fast)

    // Case A: Simple Integer (Factor -> Term -> Expr -> Main)
    let (out, err, success) = env.parse("42");
    assert!(success, "Parse failed: {}", err);
    assert!(out.contains("Expr: 42"));

    // Case B: Addition (Expr)
    let (out, _, success) = env.parse("1 + 2");
    assert!(success);
    assert!(out.contains("Expr: Add(...)")); // Check action logic

    // Case C: List Syntax (Sequences, Optionals)
    let (out, _, success) = env.parse("[ myId, otherId ]");
    assert!(success);
    assert!(out.contains("List: List[head=myId"));

    // Case D: Syntax Errors
    let (_, err, success) = env.parse("[ missing_comma other ]");
    assert!(!success, "Should fail on syntax error");
    assert!(err.contains("expected `,`"), "Error message should be descriptive. Got: {}", err);

    // Case E: Nested Parens
    let (out, _, success) = env.parse("( 1 )");
    assert!(success);
    assert!(out.contains("(Expr: 1)"));
}

#[test]
fn test_backtracking_priority() {
    // Testet explizit, ob First-Sets und Backtracking (Fork) funktionieren
    let grammar = r#"
        grammar Ambiguity {
            rule main -> String =
                  a:specific() -> { "Specific".to_string() }
                | b:general()  -> { "General".to_string() }

            // Fängt mit "fn" an
            rule specific -> () = "fn" "name" -> { () }
            
            // Fängt AUCH mit "fn" an (Identifier "fn" wäre invalide in Rust, 
            // aber "fn_call" als ident startet mit f...)
            // Wir nutzen hier Token-Ebene.
            // Sagen wir: 'ident' vs 'specific keyword'.
            
            rule general -> () = i:ident() -> { () }
        }
    "#;
    
    // Für diesen Test brauchen wir einen "klugen" Runner, der weiß, dass "fn" ein Keyword ist
    // und "fn_something" ein Ident. 
    // Da unser Parser auf `syn` basiert, ist "fn" immer ein Token![fn].
    // Ein Ident darf nicht "fn" sein.
    // Daher testen wir lieber: "let" x = 1 vs "let" = 2 (nonsense, aber gleicher Start)
    
    let grammar_v2 = r#"
        grammar Backtrack {
            rule main -> String =
                  "test" "A" -> { "Path A".to_string() }
                | "test" "B" -> { "Path B".to_string() }
        }
    "#;

    let env = TestEnv::new("Backtrack", grammar_v2);

    let (out, _, _) = env.parse("test A");
    assert!(out.contains("Path A"));

    let (out, _, _) = env.parse("test B");
    assert!(out.contains("Path B"));
}

