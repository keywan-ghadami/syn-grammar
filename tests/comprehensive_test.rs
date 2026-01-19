use syn_grammar::grammar;
use syn_grammar::testing::Testable; // Das Helper-Trait für .test()
use syn::parse::Parser; // Nötig für .parse_str()

// --- Test 1: Basis-Sequenz ---
// Prüft einfache Token-Abfolgen und Fehlerbehandlung
#[test]
fn test_basic_sequence() {
    grammar! {
        grammar Basic {
            rule main -> String = "hello" "world" -> { 
                "Success".to_string() 
            }
        }
    }

    // Zugriff erfolgt nun über das generierte Modul 'Basic'
    let res = Basic::parse_main.parse_str("hello world").test().assert_success();
    assert_eq!(res, "Success");

    let err = Basic::parse_main.parse_str("hello universe").test().assert_failure();
    // Prüfen, ob der Fehler an der richtigen Stelle auftritt
    assert!(err.to_string().contains("expected `world`"));
}

// --- Test 2: Backtracking & Priorität ---
// Prüft, ob längere Matches bevorzugt werden (wenn zuerst gelistet)
// und ob Backtracking funktioniert, wenn ein Pfad fehlschlägt.
#[test]
fn test_backtracking_priority() {
    grammar! {
        grammar Backtrack {
            rule main -> String = 
                "A" "B" -> { "Path AB".to_string() }
              | "A"     -> { "Path A".to_string() }
        }
    }

    // Fall 1: Input "A B" sollte den ersten Pfad nehmen
    let res_ab = Backtrack::parse_main.parse_str("A B").test().assert_success();
    assert_eq!(res_ab, "Path AB");

    // Fall 2: Input "A" scheitert im ersten Pfad (erwartet "B"), 
    // das System sollte zurückrollen und Pfad 2 probieren.
    let res_a = Backtrack::parse_main.parse_str("A").test().assert_success();
    assert_eq!(res_a, "Path A");
}

// --- Test 3: Komplexe Gruppen & Optionalität ---
// Prüft verschachtelte Klammerung (...) und das Fragezeichen ?
#[test]
fn test_complex_groups() {
    grammar! {
        grammar Complex {
            // (A B)? C -> Wenn A da ist, muss B folgen. Wenn nicht, direkt C.
            rule main -> String = ("A" "B")? "C" -> { "OK".to_string() }
        }
    }

    // Fall 1: Volle Sequenz
    Complex::parse_main.parse_str("A B C").test().assert_success();
    
    // Fall 2: Optionaler Teil weggelassen
    Complex::parse_main.parse_str("C").test().assert_success();
    
    // Fall 3: A da, aber B fehlt -> Fehler innerhalb der Gruppe "A B", bevor C geprüft wird
    // Das Backtracking bricht hier ab, weil "A" gematcht hat, aber "B" fehlte.
    Complex::parse_main.parse_str("A C").test().assert_failure();
}

// --- Test 4: Mathematische Ausdrücke (Rekursion & Bindings) ---
// Der wichtigste Test: Prüft rekursive Regeln, Operator-Präzedenz 
// und die korrekte Weitergabe von echten Integer-Werten (keine Strings!).
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

    // Punkt-vor-Strich Rechnung
    let val = Math::parse_main.parse_str("2 + 3 * 4").test().assert_success();
    assert_eq!(val, 14); 

    // Klammerung ändern Präzedenz
    let val2 = Math::parse_main.parse_str("(2 + 3) * 4").test().assert_success();
    assert_eq!(val2, 20);
}

// --- Test 5: Wiederholungen & Token-Klammern ---
// Prüft [ ... ] Syntax für syn::bracketed! und den * Operator
#[test]
fn test_repetition() {
    grammar! {
        grammar Repeat {
            // [ ... ] erzeugt automatisch einen Scope für bracketed!
            rule main -> usize = [ content:elems ] -> { content }

            // first:elem stellt sicher, dass mindestens 1 Element da ist,
            // rest:elem* sammelt 0 bis n weitere Elemente in einem Vec.
            rule elems -> usize = 
                first:elem rest:elem* -> { 1 + rest.len() }
            
            rule elem -> () = "x" ","? -> { () }
        }
    }

    // [ x ] -> 1 Element
    let c1 = Repeat::parse_main.parse_str("[ x ]").test().assert_success();
    assert_eq!(c1, 1);
    
    // [ x, x, x ] -> 3 Elemente
    let c3 = Repeat::parse_main.parse_str("[ x, x, x ]").test().assert_success();
    assert_eq!(c3, 3);
    
    // Leere Klammern -> Fehler (weil 'first:elem' fehlt)
    Repeat::parse_main.parse_str("[ ]").test().assert_failure();
    
    // Fehlende schließende Klammer
    let err = Repeat::parse_main.parse_str("[ x, x").test().assert_failure();
    assert!(err.to_string().contains("expected `]`"));
}

// --- Test 6: Built-ins ---
// Prüft die eingebauten Helper 'ident' und 'string_lit'
#[test]
fn test_builtins() {
    grammar! {
        grammar Builtins {
            // Rückgabe eines Tupels (String, String)
            rule main -> (String, String) = 
                k:ident "=" v:string_lit -> { (k.to_string(), v) }
        }
    }

    let (key, value) = Builtins::parse_main.parse_str("config_key = \"some_value\"").test().assert_success();
    
    assert_eq!(key, "config_key");
    assert_eq!(value, "some_value");
}
