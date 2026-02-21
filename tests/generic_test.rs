use syn::parse::Parser;
use syn_grammar::grammar;
use syn_grammar::testing::Testable;

grammar! {
    grammar generic_test {
        rule list<T>(item) -> Vec<T> =
            items:item* -> { items }

        pub rule main -> Vec<i32> =
            l:list<i32>(i32) -> { l }
    }
}

#[test]
fn test_generic_rule() {
    generic_test::parse_main
        .parse_str("1 2 3")
        .test()
        .assert_success_is(vec![1, 2, 3]);
}

grammar! {
    grammar generic_map {
        use std::collections::HashMap;

        rule map<K: std::hash::Hash + Eq, V>(k, v) -> HashMap<K, V> =
            entries:entry<K, V>(k, v)* -> {
                entries.into_iter().collect()
            }

        rule entry<K, V>(k, v) -> (K, V) =
            key:k ":" val:v -> { (key, val) }

        pub rule main -> HashMap<String, i32> =
            m:map<String, i32>(string, i32) -> {
                m.into_iter().map(|(k, v)| (k.value, v)).collect()
            }
    }
}

#[test]
fn test_generic_map() {
    let mut expected = std::collections::HashMap::new();
    expected.insert("a".to_string(), 1);
    expected.insert("b".to_string(), 2);

    generic_map::parse_main
        .parse_str("\"a\": 1 \"b\": 2")
        .test()
        .assert_success_is(expected);
}
