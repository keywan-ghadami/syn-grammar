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

/// The builtin catalogue (`backend.rs`) declares a return type for every
/// builtin. `monomorphize::infer_type` reads exactly that entry to determine the
/// generic parameter of a rule - a wrong entry therefore produces a compiler
/// error in the *generated* code, not at the call site.
///
/// `digit`, `hex_digit` and `oct_digit` were declared as `syn::Ident` but
/// return `syn::LitInt` (`token_filter.rs`). This test binds the three via a
/// generic rule and fails on a wrong catalogue entry before any user stumbles
/// over it.
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

    let werte = digit_generics::parse_dezimal
        .parse_str("1 2 3")
        .test()
        .assert_success();
    let gelesen: Vec<String> = werte
        .iter()
        .map(|l| l.base10_digits().to_string())
        .collect();
    assert_eq!(gelesen, vec!["1", "2", "3"]);

    digit_generics::parse_hexadezimal
        .parse_str("10 11")
        .test()
        .assert_success();
    digit_generics::parse_oktal
        .parse_str("7 5")
        .test()
        .assert_success();
}
