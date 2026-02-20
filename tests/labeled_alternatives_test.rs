use syn::parse::Parser;
use syn_grammar::grammar;
use syn_grammar::testing::Testable;

#[test]
fn test_default_labels() {
    grammar! {
        grammar default_labels {
            pub rule main -> () = (a | b) -> { () }
            rule a -> () = "a" -> { () }
            rule b -> () = "b" -> { () }
        }
    }

    let err = default_labels::parse_main
        .parse_str("x")
        .test()
        .assert_failure();
    // We expect "expected one of: a, b" or similar.
    println!("Error: {}", err);
    assert!(err.to_string().contains("expected one of: a, b"));
}

#[test]
fn test_explicit_labels() {
    grammar! {
        grammar explicit_labels {
            pub rule main -> ()
                = "a" # "Letter A" -> { () }
                | "b" # "Letter B" -> { () }
        }
    }

    let err = explicit_labels::parse_main
        .parse_str("x")
        .test()
        .assert_failure();
    println!("Error: {}", err);
    assert!(err
        .to_string()
        .contains("expected one of: Letter A, Letter B"));
}

#[test]
fn test_deep_error_wins() {
    grammar! {
        grammar deep_error {
            pub rule main -> ()
                = "a" "b" # "AB" -> { () }
                | "c" # "C" -> { () }
        }
    }

    // Input "a x" matches first part of AB, fails at "b". This is deep.
    // So error should be "expected 'b'", NOT "expected one of: AB, C".
    let err = deep_error::parse_main
        .parse_str("a x")
        .test()
        .assert_failure();
    println!("Error: {}", err);
    assert!(err.to_string().contains("expected `b`"));
    assert!(!err.to_string().contains("expected one of:"));
}

#[test]
fn test_group_labels() {
    grammar! {
        grammar group_labels {
            pub rule main -> ()
                = ("a" # "A" | "b" # "B") -> { () }
        }
    }

    let err = group_labels::parse_main
        .parse_str("x")
        .test()
        .assert_failure();
    println!("Error: {}", err);
    assert!(err.to_string().contains("expected one of: A, B"));
}
