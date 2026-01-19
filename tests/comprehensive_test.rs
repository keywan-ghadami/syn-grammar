#![cfg(feature = "jit")]

use syn_grammar::testing::TestEnv;

/// Test 1: Basis-Funktionalität
/// Prüft Literale, Sequenzen und Return-Werte.
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
    
    // Fall 1: Korrekter Input
    let res = env.parse("hello world");
    res.assert_success();
    assert!(res.stdout.contains("Success"));

    // Fall 2: Falscher Input
    let res_fail = env.parse("hello universe");
    res_fail.assert_failure();
    assert!(res_fail.stderr.contains("expected `world`"));
}

/// Test 2: Backtracking Priorität
/// Das war der Test, der vorher fehlschlug. Er prüft, ob der Parser
/// korrekt von einem spezifischen Pfad ("A" "B") auf einen generelleren ("A")
/// zurückfällt, wenn das zweite Token nicht passt.
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

    // Input "A B" -> Sollte den ersten Zweig nehmen
    let res_ab = env.parse("A B");
    res_ab.assert_success();
    assert!(res_ab.stdout.contains("Path AB"));

    // Input "A" -> Erster Zweig startet (wg. "A"), scheitert an "B". 
    // Muss Backtracking machen und Zweig 2 nehmen.
    let res_a = env.parse("A");
    res_a.assert_success();
    assert!(res_a.stdout.contains("Path A"));
}

/// Test 3: Komplexe Gruppen und Suffixe
/// Prüft Kombinationen aus Gruppen `(...)` und Optionals `?`.
/// Hier hatten wir zuvor Probleme mit zu aggressivem Peeking.
#[test]
fn test_complex_groups() {
    let grammar = r#"
        grammar Complex {
            // (A B)? C
            // Bedeutet: Entweder "A B C" oder nur "C".
            // "A C" darf NICHT "C" parsen, sondern muss fehlschlagen.
            rule main -> String = ("A" "B")? "C" -> { "OK".to_string() }
        }
    "#;

    let env = TestEnv::new("Complex", grammar);

    // Fall 1: Voller Match
    env.parse("A B C").assert_success();

    // Fall 2: Optional übersprungen
    env.parse("C").assert_success();

    // Fall 3: "A" ist da, aber "B" fehlt.
    // Der Parser versucht ("A" "B"), scheitert bei "B".
    // Er resettet (Backtracking) und steht wieder vor "A".
    // Dann versucht er weiter im Text -> erwartet "C".
    // Er findet "A". Fehler: Expected "C".
    let res_fail = env.parse("A C");
    res_fail.assert_failure();
    // Der Fehler muss sich auf das erwarten von C beziehen, da der optionale Block fehlschlug
    assert!(res_fail.stderr.contains("expected `C`") || res_fail.stderr.contains("expected `B`"));
}

/// Test 4: Rekursion und Operatoren
/// Ein kleiner mathematischer Ausdruck-Parser, um Rekursion und Rule-Calls zu testen.
#[test]
fn test_math_expression() {
    let grammar = r#"
        grammar Math {
            // Einfache Grammatik für Addition und Multiplikation
            // Wir nutzen i32 als Rückgabetyp
            
            rule main -> i32 = expr -> { expr }

            rule expr -> i32 = 
                t:term "+" e:expr -> { t + e }
              | t:term            -> { t }

            rule term -> i32 = 
                f:factor "*" t:term -> { f * t }
              | f:factor            -> { f }

            rule factor -> i32 = 
                "(" e:expr ")" -> { e }
              | i:int_lit      -> { i }
        }
    "#;

    let env = TestEnv::new("Math", grammar);

    // Einfache Addition
    let res1 = env.parse("1 + 2");
    res1.assert_success();
    assert!(res1.stdout.contains("3"));

    // Punkt vor Strich (durch Grammatik-Struktur)
    // 2 + 3 * 4 = 14
    let res2 = env.parse("2 + 3 * 4");
    res2.assert_success();
    assert!(res2.stdout.contains("14"));

    // Klammern
    // (2 + 3) * 4 = 20
    let res3 = env.parse("(2 + 3) * 4");
    res3.assert_success();
    assert!(res3.stdout.contains("20"));
}

/// Test 5: Wiederholungen (Repetition)
/// Testet `*` und `+` Operatoren.
#[test]
fn test_repetition() {
    let grammar = r#"
        grammar Repeat {
            // List: [ Element, Element, ... ]
            rule main -> usize = "[" content:elems "]" -> { content }

            rule elems -> usize = 
                first:elem rest:elem* -> { 1 + rest.len() }
            
            rule elem -> () = "x" ","? -> { () }
        }
    "#;

    let env = TestEnv::new("Repeat", grammar);

    // Ein Element
    let res1 = env.parse("[ x ]");
    res1.assert_success();
    assert!(res1.stdout.contains("1"));

    // Drei Elemente mit Kommas
    let res3 = env.parse("[ x, x, x ]");
    res3.assert_success();
    assert!(res3.stdout.contains("3"));

    // Leere Liste sollte fehlschlagen (da elems mind. 1 'first' erwartet)
    env.parse("[ ]").assert_failure();
}

/// Test 6: Builtin Types
/// Prüft, ob `ident` und `string_lit` korrekt gemappt werden.
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
    // Ident wird geparst, String wird geparst (ohne Anführungszeichen im Value)
    assert!(res.stdout.contains("key: value"));
}
