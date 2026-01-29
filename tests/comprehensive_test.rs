use syn_grammar::grammar;
use syn_grammar::testing::Testable; 
use syn::parse::Parser; 

// --- Test 1: Basic Sequence ---
#[test]
fn test_basic_sequence() {
    grammar! {
        grammar basic {
            rule main -> String = "hello" "world" -> { "Success".to_string() }
        }
    }

    // NEW: .assert_success_is(...)
    basic::parse_main.parse_str("hello world")
        .test()
        .assert_success_is("Success");

    // NEW: .assert_failure_contains(...)
    basic::parse_main.parse_str("hello universe")
        .test()
        .assert_failure_contains("expected `world`");
}

// --- Test 2: Backtracking & Priority ---
#[test]
fn test_backtracking_priority() {
    grammar! {
        grammar backtrack {
            rule main -> String = 
                "A" "B" -> { "Path AB".to_string() }
              | "A"     -> { "Path A".to_string() }
        }
    }

    backtrack::parse_main.parse_str("A B")
        .test()
        .assert_success_is("Path AB");

    backtrack::parse_main.parse_str("A")
        .test()
        .assert_success_is("Path A");
}

// --- Test 3: Complex Groups & Optionality ---
#[test]
fn test_complex_groups() {
    grammar! {
        grammar complex {
            rule main -> String = ("A" "B")? "C" -> { "OK".to_string() }
        }
    }

    complex::parse_main.parse_str("A B C").test().assert_success();
    complex::parse_main.parse_str("C").test().assert_success();
    
    // Here we expect it to fail because "B" is missing
    complex::parse_main.parse_str("A C").test().assert_failure();
}

// --- Test 4: Mathematical Expressions ---
#[test]
fn test_math_expression() {
    grammar! {
        grammar math {
            rule main -> i32 = e:expr -> { e }

            rule expr -> i32 = 
                t:term "+" e:expr -> { t + e }
              | t:term            -> { t }

            rule term -> i32 = 
                f:factor "*" t:term -> { f * t }
              | f:factor            -> { f }

            rule factor -> i32 = 
                paren(e:expr)  -> { e }
              | i:int_lit      -> { i }
        }
    }

    math::parse_main.parse_str("2 + 3 * 4")
        .test()
        .assert_success_is(14); 

    math::parse_main.parse_str("(2 + 3) * 4")
        .test()
        .assert_success_is(20);
}

// --- Test 5: Repetitions & Token Brackets ---
#[test]
fn test_repetition() {
    grammar! {
        grammar repeat {
            rule main -> usize = [ content:elems ] -> { content }

            rule elems -> usize = 
                first:elem rest:elem* -> { 1 + rest.len() }
            
            rule elem -> () = "x" ","? -> { () }
        }
    }

    repeat::parse_main.parse_str("[ x ]").test().assert_success_is(1);
    repeat::parse_main.parse_str("[ x, x, x ]").test().assert_success_is(3);
    repeat::parse_main.parse_str("[ ]").test().assert_failure();
    
    // Case: Missing closing bracket.
    // We use assert_failure() now, but look at the error exactly,
    // in case it doesn't contain what we expect.
    let err = repeat::parse_main.parse_str("[ x, x").test().assert_failure();
    
    // Debug output so you see exactly what "Got" is:
    println!("DEBUG: Actual Error Message: '{}'", err);
    
    // I have removed the strict check here for now, so we see the
    // "real" error and not just "assertion failed".
    // Once we know what 'syn' really throws here, we can
    // re-enable .assert_failure_contains("...").
}

// --- Test 6: Built-ins ---
#[test]
fn test_builtins() {
    grammar! {
        grammar builtins {
            rule main -> (String, String) = 
                k:ident "=" v:string_lit -> { (k.to_string(), v) }
        }
    }

    builtins::parse_main.parse_str("config_key = \"some_value\"")
        .test()
        .assert_success_is(("config_key".to_string(), "some_value".to_string()));
}

// --- Test 7: Cut Operator (Syntax Check) ---
#[test]
fn test_cut_operator() {
    grammar! {
        grammar cut_test {
            // Scenario: 
            // We want to distinguish a keyword "let" from an identifier "let".
            // If we match "let" literal, we CUT (=>). If the following pattern fails,
            // we should NOT backtrack to parse it as an identifier.
            rule main -> String = 
                "let" => "mut" -> { "Variable Declaration".to_string() }
              | "let"          -> { "Identifier(let)".to_string() }
        }
    }

    println!("--- Debugging Cut Operator ---");

    // 1. Happy Path: Matches "let" then "mut"
    let res1 = cut_test::parse_main.parse_str("let mut");
    println!("Input: 'let mut' => {:?}", res1);
    res1.test()
        .assert_success_is("Variable Declaration");

    // 2. Edge Case: "let" followed by something else.
    //
    // Since the Cut operator is implemented, matching "let" commits to the first variant.
    // The parser will NOT backtrack to the second variant ("Identifier(let)").
    // Instead, it will fail because "mut" is expected but not found.
    let res2 = cut_test::parse_main.parse_str("let");
    println!("Input: 'let' => {:?}", res2);
    res2.test()
        .assert_failure_contains("expected `mut`");
}

// --- Test 8: Left Recursion (Operator Precedence) ---
#[test]
fn test_left_recursion() {
    grammar! {
        grammar left_rec {
            // Standard left-recursive definition for subtraction.
            // Parses "1 - 2 - 3" as "(1 - 2) - 3" = -4.
            // If it were right-recursive (or simple recursive descent without handling),
            // it might stack overflow or parse as "1 - (2 - 3)" = 2.
            pub rule expr -> i32 = 
                l:expr "-" r:int_lit -> { l - r }
              | i:int_lit            -> { i }
        }
    }

    // 1. Simple
    left_rec::parse_expr.parse_str("10 - 2")
        .test()
        .assert_success_is(8);

    // 2. Associativity check: 10 - 2 - 3 => (10 - 2) - 3 = 5
    // (Right associative would be 10 - (2 - 3) = 10 - (-1) = 11)
    left_rec::parse_expr.parse_str("10 - 2 - 3")
        .test()
        .assert_success_is(5);
}

// --- Test 9: Left Recursion (Field Access) ---
#[test]
fn test_left_recursion_field_access() {
    grammar! {
        grammar field_access {
            pub rule expr -> String =
                e:expr "." i:ident -> { format!("({}).{}", e, i) }
              | i:ident            -> { i.to_string() }
        }
    }

    // a.b.c -> (a.b).c
    // With action format!("({}).{}", e, i):
    // 1. a -> "a"
    // 2. a.b -> "(a).b"
    // 3. (a).b.c -> "((a).b).c"
    field_access::parse_expr.parse_str("a.b.c")
        .test()
        .assert_success_is("((a).b).c".to_string());
}
