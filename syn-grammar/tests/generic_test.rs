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
