use syn_grammar::grammar;
use syn_grammar::testing::Testable; // Unser neues Trait
use syn::parse::Parser; // Nötig für .parse_str()

// --- Test 1: Basis-Sequenz ---
#[test]
fn test_basic_sequence() {
    // 1. Definiere die Grammatik direkt hier (wird zur Compile-Zeit expandiert!)
    grammar! {
        grammar Basic {
            rule main -> String = "hello" "world" -> { 
                "Success".to_string() 
            }
        }
    }

    // 2. Nutze die generierte Funktion 'parse_main'
    // Der Generator erzeugt 'parse_main', das Parser-Methoden wie parse_str hat.
    
    // Erfolgsfall
    let res = parse_main.parse_str("hello world").test().assert_success();
    assert_eq!(res, "Success");

    // Fehlerfall
    let err = parse_main.parse_str("hello universe").test().assert_failure();
    assert!(err.to_string().contains("expected `world`"));
}

// --- Test 2: Math Expression (mit Rückgabewerten prüfen!) ---
#[test]
fn test_math_expression() {
    grammar! {
        grammar Math {
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

    // Jetzt können wir echte Integers prüfen, kein String-Parsing des Outputs mehr!
    let val = parse_main.parse_str("2 + 3 * 4").test().assert_success();
    assert_eq!(val, 14); // Echter Integer-Vergleich!

    let val2 = parse_main.parse_str("(2 + 3) * 4").test().assert_success();
    assert_eq!(val2, 20);
}
