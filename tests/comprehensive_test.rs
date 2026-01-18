mod common;
use common::TestEnv;

#[test]
fn test_complex_grammar_features() {
    let grammar = r#"
        grammar Comprehensive {
            rule main -> String = 
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

            // KORREKTUR: Hier nutzen wir [ und ] OHNE Anführungszeichen.
            // Das triggert Pattern::Bracketed im Parser statt Pattern::Lit.
            rule list -> String = 
                [ 
                    head:ident() 
                    ( "," tail:ident() )* ","? 
                ] 
                -> { 
                    format!("List[head={}]", head) 
                }
            
            rule grouped -> i32 =
                ( "a" | "b" ) "c" -> { 0 }
        }
    "#;

    let env = TestEnv::new("Comprehensive", grammar);

    let (out, err, success) = env.parse("42");
    assert!(success, "Parse failed: {}", err);
    assert!(out.contains("Expr: 42"));

    let (out, _, success) = env.parse("1 + 2");
    assert!(success);
    assert!(out.contains("Expr: Add(...)")); 

    // Hier nutzen wir echte Bracket-Syntax im Input
    let (out, _, success) = env.parse("[ myId, otherId ]");
    assert!(success, "List parsing failed");
    assert!(out.contains("List: List[head=myId]"));

    let (_, err, success) = env.parse("[ missing_comma other ]");
    assert!(!success, "Should fail on syntax error");
    assert!(err.contains("expected `,`") || err.contains("expected `]`"), "Got: {}", err);

    let (out, _, success) = env.parse("paren( 1 )");
    assert!(success);
    assert!(out.contains("(Expr: 1)"));
}

#[test]
fn test_backtracking_priority() {
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

#[test]
fn test_keyword_collisions() {
    let grammar = r#"
        grammar Keywords {
            rule main -> String = 
                "function" name:ident() 
                -> { name.to_string() }
        }
    "#;
    
    let env = TestEnv::new("Keywords", grammar);

    let (out, _, success) = env.parse("function myFunc");
    assert!(success);
    assert!(out.contains("myFunc"));

    let (out, _, success) = env.parse("function fn");
    assert!(success, "Should accept 'fn' as identifier");
    assert!(out.contains("fn"));

    let (out, _, success) = env.parse("function struct");
    assert!(success);
    assert!(out.contains("struct"));
}
