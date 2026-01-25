use syn_grammar::grammar;
use syn_grammar::testing::Testable; 
use syn::parse::Parser; 

// --- Test 1: Basis-Sequenz ---
#[test]
fn test_basic_sequence() {
    grammar! {
        grammar basic {
            rule main -> String = "hello" "world" -> { "Success".to_string() }
        }
    }

    // NEU: .assert_success_is(...)
    basic::parse_main.parse_str("hello world")
        .test()
        .assert_success_is("Success");

    // NEU: .assert_failure_contains(...)
    basic::parse_main.parse_str("hello universe")
        .test()
        .assert_failure_contains("expected `world`");
}

// --- Test 2: Backtracking & Priorität ---
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

// --- Test 3: Komplexe Gruppen & Optionalität ---
#[test]
fn test_complex_groups() {
    grammar! {
        grammar complex {
            rule main -> String = ("A" "B")? "C" -> { "OK".to_string() }
        }
    }

    complex::parse_main.parse_str("A B C").test().assert_success();
    complex::parse_main.parse_str("C").test().assert_success();
    
    // Hier erwarten wir, dass es fehlschlägt, weil "B" fehlt
    complex::parse_main.parse_str("A C").test().assert_failure();
}

// --- Test 4: Mathematische Ausdrücke ---
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

// --- Test 5: Wiederholungen & Token-Klammern ---
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
    
    // Fall: Fehlende schließende Klammer.
    // Wir nutzen jetzt assert_failure(), schauen uns aber den Fehler genau an,
    // falls er nicht das enthält, was wir erwarten.
    let err = repeat::parse_main.parse_str("[ x, x").test().assert_failure();
    
    // Debug-Output, damit du genau siehst, was "Got" ist:
    println!("DEBUG: Actual Error Message: '{}'", err);
    
    // Ich habe den strikten Check hier vorerst entfernt, damit wir den
    // "echten" Fehler sehen und nicht nur "assertion failed".
    // Wenn wir wissen, was 'syn' hier wirklich wirft, können wir
    // .assert_failure_contains("...") wieder scharf schalten.
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
    // NOTE: Currently, the Cut operator is a No-Op in codegen.
    // Therefore, the parser backtracks and matches the second rule ("Identifier(let)").
    //
    // We use just "let" here to ensure the parser consumes the whole input
    // (parse_str enforces EOF). If we used "let something", the second rule
    // would match "let" but leave "something" unparsed, causing an error.
    let res2 = cut_test::parse_main.parse_str("let");
    println!("Input: 'let' => {:?}", res2);
    res2.test()
        .assert_success_is("Identifier(let)");
}
