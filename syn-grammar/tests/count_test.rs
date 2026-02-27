use syn::parse::Parser;
use syn_grammar::grammar; // Import Parser trait

grammar! {
    grammar count_test {
        pub rule count_stars -> usize = c:count("a"*) -> { c }
        pub rule count_plus -> usize = c:count("b"+) -> { c }
        pub rule count_opts -> usize = c:count("c"?) -> { c }
        pub rule count_group -> usize = c:count(("d" "e")*) -> { c }

        // Count with bindings (should be ignored)
        pub rule count_bindings -> usize = c:count(x:"f"*) -> { c }
    }
}

#[test]
fn test_count_stars() {
    let code = "a a a";
    let count = count_test::parse_count_stars.parse_str(code).unwrap();
    assert_eq!(count, 3);

    let code = "";
    let count = count_test::parse_count_stars.parse_str(code).unwrap();
    assert_eq!(count, 0);
}

#[test]
fn test_count_plus() {
    let code = "b b b";
    let count = count_test::parse_count_plus.parse_str(code).unwrap();
    assert_eq!(count, 3);

    let code = "b";
    let count = count_test::parse_count_plus.parse_str(code).unwrap();
    assert_eq!(count, 1);
}

#[test]
fn test_count_plus_fail() {
    let code = "";
    let res = count_test::parse_count_plus.parse_str(code);
    assert!(res.is_err());
}

#[test]
fn test_count_opts() {
    let code = "c";
    let count = count_test::parse_count_opts.parse_str(code).unwrap();
    assert_eq!(count, 1);

    let code = "";
    let count = count_test::parse_count_opts.parse_str(code).unwrap();
    assert_eq!(count, 0);
}

#[test]
fn test_count_group() {
    let code = "d e d e d e";
    let count = count_test::parse_count_group.parse_str(code).unwrap();
    assert_eq!(count, 3);
}

#[test]
fn test_count_bindings() {
    let code = "f f";
    let count = count_test::parse_count_bindings.parse_str(code).unwrap();
    assert_eq!(count, 2);
}
