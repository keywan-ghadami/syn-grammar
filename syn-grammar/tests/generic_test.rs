use std::collections::HashMap;
use syn::parse::Parser;
use syn_grammar::grammar;
use syn_grammar::testing::Testable;

#[test]
fn test_generic_rule() {
    grammar! {
        grammar generic_list {
            rule list<T>(item) -> Vec<T> =
                items:item* -> { items }

            pub rule main -> Vec<i32> =
                l:list<i32>(i32) -> { l }
        }
    }

    generic_list::parse_main
        .parse_str("1 2 3")
        .test()
        .assert_success_is(vec![1, 2, 3]);
}

#[test]
fn test_generic_inference() {
    grammar! {
        grammar generic_map {
            rule map<K: Hash + Eq, V>(k, v) -> HashMap<K, V> =
                entries:entry(k=k, v=v)* -> { entries.into_iter().collect() }

            rule entry<K, V>(k, v) -> (K, V) =
                key:k ":" val:v -> { (key, val) }

            // Using named arguments to allow inference without explicit generics
            pub rule main -> HashMap<String, i32> =
                m:map(k=string, v=i32) -> {
                    m.into_iter().map(|(k, v)| (k.value, v)).collect()
                }
        }
    }
    let mut expected = HashMap::new();
    expected.insert("a".to_string(), 1);
    expected.insert("b".to_string(), 2);

    generic_map::parse_main
        .parse_str(r#""a": 1 "b": 2"#)
        .test()
        .assert_success_is(expected);
}

/// Der Builtin-Katalog (`backend.rs`) deklariert fuer jedes Builtin einen
/// Rueckgabetyp. `monomorphize::infer_type` liest genau diesen Eintrag, um den
/// Generic-Parameter einer Regel zu bestimmen - ein falscher Eintrag erzeugt
/// deshalb einen Compilerfehler im *generierten* Code, nicht an der Aufrufstelle.
///
/// `digit`, `hex_digit` und `oct_digit` waren als `syn::Ident` deklariert,
/// liefern aber `syn::LitInt` (`token_filter.rs`). Dieser Test bindet die drei
/// ueber eine generische Regel ein und schlaegt bei einem falschen Katalogeintrag
/// fehl, bevor irgendein Nutzer darueber stolpert.
#[test]
fn generische_regel_mit_token_filtern() {
    grammar! {
        grammar digit_generics {
            rule liste<T>(item) -> Vec<T> = items:item* -> { items }

            pub rule dezimal -> Vec<syn::LitInt> = l:liste(item=digit) -> { l }
            pub rule hexadezimal -> Vec<syn::LitInt> = l:liste(item=hex_digit) -> { l }
            pub rule oktal -> Vec<syn::LitInt> = l:liste(item=oct_digit) -> { l }
        }
    }

    let werte = digit_generics::parse_dezimal.parse_str("1 2 3").test().assert_success();
    let gelesen: Vec<String> = werte.iter().map(|l| l.base10_digits().to_string()).collect();
    assert_eq!(gelesen, vec!["1", "2", "3"]);

    digit_generics::parse_hexadezimal.parse_str("10 11").test().assert_success();
    digit_generics::parse_oktal.parse_str("7 5").test().assert_success();
}
