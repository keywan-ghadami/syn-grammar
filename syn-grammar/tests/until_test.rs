use syn::parse::Parser;
use syn_grammar::grammar; // Import Parser trait

grammar! {
    grammar until_test {
        pub rule main -> (String, String)
            = body:until(";") ";" -> { (body.to_string(), ";".to_string()) }

        pub rule fn_keyword -> () = "fn" -> { () }

        pub rule until_keyword -> (String, String)
            = body:until(fn_keyword) "fn" -> { (body.to_string(), "fn".to_string()) }

        pub rule nested_until -> String
            = "start" body:until("end") "end" -> { body.to_string() }

        pub rule ab -> String = "a" -> { "a".to_string() } | "b" -> { "b".to_string() }

        pub rule until_group -> String
             = body:until(ab) delim:ab -> { format!("{}{}", body, delim) }
    }
}

// use until_test::*;

fn parse_main_wrapper(input: &str) -> syn::Result<(String, String)> {
    until_test::parse_main.parse_str(input)
}

fn parse_until_keyword_wrapper(input: &str) -> syn::Result<(String, String)> {
    until_test::parse_until_keyword.parse_str(input)
}

fn parse_nested_until_wrapper(input: &str) -> syn::Result<String> {
    until_test::parse_nested_until.parse_str(input)
}

fn parse_until_group_wrapper(input: &str) -> syn::Result<String> {
    until_test::parse_until_group.parse_str(input)
}

#[test]
fn test_until_semicolon() {
    let input = "hello world ;";
    let (body, delim) = parse_main_wrapper(input).unwrap();
    assert_eq!(body.trim(), "hello world");
    assert_eq!(delim, ";");

    // Empty body
    let input = ";";
    let (body, delim) = parse_main_wrapper(input).unwrap();
    assert_eq!(body.trim(), "");
    assert_eq!(delim, ";");
}

#[test]
fn test_until_keyword() {
    let input = "pub struct X fn";
    let (body, delim) = parse_until_keyword_wrapper(input).unwrap();
    assert_eq!(body.trim(), "pub struct X");
    assert_eq!(delim, "fn");
}

#[test]
fn test_nested_until() {
    let input = "start some content end";
    let body = parse_nested_until_wrapper(input).unwrap();
    assert_eq!(body.trim(), "some content");
}

#[test]
fn test_until_group() {
    let input = "x y z a";
    let res = parse_until_group_wrapper(input).unwrap();
    // println!("Res: {}", res);
    assert!(res.contains("x"));
    assert!(res.contains("y"));
    assert!(res.contains("z"));
    assert!(res.ends_with("a"));

    let input = "x y z b";
    let res = parse_until_group_wrapper(input).unwrap();
    assert!(res.ends_with("b"));
}
