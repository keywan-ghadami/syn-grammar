use syn_grammar::testing::TestEnv;

#[test]
fn test_complex_grammar_features() {
    let grammar = r#"
        grammar Comprehensive {
            pub rule main -> String = 
                  e:expr() -> { format!("Expr: {}", e) }
                | l:list() -> { format!("List: {}", l) }

            rule expr -> String = 
                t:term() ( "+" t2:term() )* -> { "Add(...)".to_string() }

            rule term -> String = 
                f:factor() ( "*" f2:factor() )* -> { "Mul(...)".to_string() }

            rule factor -> String = 
                  i:int_lit() -> { i.to_string() }
                | paren( e:expr() ) -> { format!("({})", e) } 
                | "nop" -> { "Nop".to_string() }

            // Test für Listen und Bracket-Syntax [ ... ]
            rule list -> String = 
                [ 
                    head:ident() 
                    ( "," tail:ident() )* ","? 
                ] 
                -> { 
                    format!("List[head={}]", head) 
                }
            
            // Test für logisches Grouping ( | )
            rule grouped -> i32 =
                ( "a" | "b" ) "c" -> { 0 }
        }
    "#;

    // Kompiliert den Parser on-the-fly
    let env = TestEnv::new("Comprehensive", grammar);

    // Test 1: Einfacher Ausdruck
    let (out, _, success) = env.parse("42");
    assert!(success);
    assert!(out.contains("Expr: 42"));

    // Test 2: Rekursion und Operatoren
    let (out, _, success) = env.parse("1 + 2");
    assert!(success);
    assert!(out.contains("Expr: Add(...)")); 

    // Test 3: Listen Syntax [a, b]
    let (out, _, success) = env.parse("[ myId, otherId ]");
    assert!(success);
    assert!(out.contains("List: List[head=myId]"));

    // Test 4: Syntax Fehler erwarten
    let (_, _, success) = env.parse("[ missing_comma other ]");
    assert!(!success, "Should fail on invalid syntax");

    // Test 5: Klammern via 'paren(...)'
    let (out, _, success) = env.parse("paren( 1 )");
    assert!(success);
    assert!(out.contains("(Expr: 1)"));
}

#[test]
fn test_backtracking_priority() {
    // Testet, ob speculatives Parsing (rt::parse_speculative) funktioniert.
    // Beide Varianten beginnen mit "test", der Parser muss forken.
    let grammar_v2 = r#"
        grammar Backtrack {
            pub rule main -> String =
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

#[test]
fn test_keyword_collisions() {
    // Testet, ob wir Rust-Keywords (wie 'fn') als Identifier nutzen können,
    // dank rt::parse_ident.
    let grammar = r#"
        grammar Keywords {
            pub rule main -> String = 
                "function" name:ident() 
                -> { name.to_string() }
        }
    "#;
    
    let env = TestEnv::new("Keywords", grammar);

    let (out, _, success) = env.parse("function myFunc");
    assert!(success);
    assert!(out.contains("myFunc"));

    // Das hier ist der kritische Test: "fn" ist ein Keyword, soll aber geparst werden.
    let (out, _, success) = env.parse("function fn");
    assert!(success, "Should accept 'fn' as identifier");
    assert!(out.contains("fn"));
}
