use syn::parse::Parser;
use syn::spanned::Spanned;
use syn_grammar::grammar;

grammar! {
    grammar test_opt {
        rule main -> bool =
            "fn"
            m:"mut"?
            name:ident
            -> { m.is_some() }
    }
}

#[test]
fn test_literal_binding_optional() {
    let input = "fn mut foo";
    let res = test_opt::parse_main.parse_str(input).unwrap();
    assert!(res);

    let input = "fn foo";
    let res = test_opt::parse_main.parse_str(input).unwrap();
    assert!(!res);
}

grammar! {
    grammar bind_lit {
        rule main -> syn::Token![fn] =
            f:"fn"
            -> { f }
    }
}

#[test]
fn test_literal_binding_direct() {
    let input = "fn";
    let res = bind_lit::parse_main.parse_str(input).unwrap();
    // It returns the token
    assert_eq!(res.span().start().line, 1);
}

grammar! {
    grammar span_bind {
        rule main -> proc_macro2::Span =
            "fn" @ s
            -> { s }
    }
}

#[test]
fn test_literal_span_binding() {
    let input = "fn";
    let res = span_bind::parse_main.parse_str(input).unwrap();
    // It returns the span
    assert_eq!(res.start().line, 1);
}
