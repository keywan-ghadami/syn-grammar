#![cfg(feature = "jit")]

use syn_grammar::testing::TestEnv;

#[test]
fn test_basic_sequence() {
    let grammar = r#"
        grammar Basic {
            rule main -> String = "hello" "world" -> { 
                "Success".to_string() 
            }
        }
    "#;

    let env = TestEnv::new("Basic", grammar);
    
    env.parse("hello world").assert_success();

    let res_fail = env.parse("hello universe");
    res_fail.assert_failure();
    assert!(!res_fail.stderr.is_empty());
}

#[test]
fn test_backtracking_priority() {
    let grammar = r#"
        grammar Backtrack {
            rule main -> String = 
                "A" "B" -> { "Path AB".to_string() }
              | "A"     -> { "Path A".to_string() }
        }
    "#;

    let env = TestEnv::new("Backtrack", grammar);

    let res_ab = env.parse("A B");
    res_ab.assert_success();
    assert!(res_ab.stdout.contains("Path AB"));

    let res_a = env.parse("A");
    res_a.assert_success();
    assert!(res_a.stdout.contains("Path A"));
}

#[test]
fn test_complex_groups() {
    let grammar = r#"
        grammar Complex {
            rule main -> String = ("A" "B")? "C" -> { "OK".to_string() }
        }
    "#;

    let env = TestEnv::new("Complex", grammar);

    env.parse("A B C").assert_success();
    env.parse("C").assert_success();

    let res_fail = env.parse("A C");
    res_fail.assert_failure();
}

#[test]
fn test_math_expression() {
    let grammar = r#"
        grammar Math {
            rule main -> i32 = expr -> { expr }

            rule expr -> i32 = 
                t:term "+" e:expr -> { t + e }
              | t:term            -> { t }

            rule term -> i32 = 
                f:factor "*" t:term -> { f * t }
              | f:factor            -> { f }

            // FIX: Use explicit tokens "(" and ")" instead of paren(...).
            // paren(...) creates a scope, so 'e' would die inside the block.
            // "(" e:expr ")" keeps 'e' in the current scope.
            rule factor -> i32 = 
                "(" e:expr ")"  -> { e }
              | i:int_lit       -> { i }
        }
    "#;

    let env = TestEnv::new("Math", grammar);

    let res1 = env.parse("1 + 2");
    res1.assert_success();
    assert!(res1.stdout.contains("3"));

    let res2 = env.parse("2 + 3 * 4");
    res2.assert_success();
    assert!(res2.stdout.contains("14"));

    let res3 = env.parse("(2 + 3) * 4");
    res3.assert_success();
    assert!(res3.stdout.contains("20"));
}

#[test]
fn test_repetition() {
    let grammar = r#"
        grammar Repeat {
            // FIX: Use "[" ... "]" tokens instead of bracketed[...]
            // This ensures 'content' is available in the action block.
            rule main -> usize = "[" content:elems "]" -> { content }

            // With the codegen update, 'rest:elem*' now accumulates into a Vec<()>.
            rule elems -> usize = 
                first:elem rest:elem* -> { 1 + rest.len() }
            
            rule elem -> () = "x" ","? -> { () }
        }
    "#;

    let env = TestEnv::new("Repeat", grammar);

    let res1 = env.parse("[ x ]");
    res1.assert_success();
    assert!(res1.stdout.contains("1"));

    let res3 = env.parse("[ x, x, x ]");
    res3.assert_success();
    assert!(res3.stdout.contains("3"));

    env.parse("[ ]").assert_failure();
}

#[test]
fn test_builtins() {
    let grammar = r#"
        grammar Builtins {
            rule main -> String = 
                k:ident "=" v:string_lit -> { format!("{}: {}", k, v) }
        }
    "#;

    let env = TestEnv::new("Builtins", grammar);

    let res = env.parse("key = \"value\"");
    res.assert_success();
    assert!(res.stdout.contains("key: value"));
}
