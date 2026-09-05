//! Self-hosting: the grammar DSL, written in the grammar DSL.
//!
//! This is the second acceptance benchmark next to `cxx-parser`. The
//! hand-written parser in `syn-grammar-model` stays the stage-zero parser
//! (the macro crate cannot depend on itself); this grammar covers the same
//! language and is checked against real grammars from the documentation.
//! Whatever the DSL cannot express about itself shows up here first.
//!
//! One thing the DSL has to say explicitly: a pattern sequence without an
//! action block runs into the next rule, because `rule` is not a Rust
//! keyword. The hand-written parser stops on a peek; here it is the
//! `not("rule")` guard in `base`.
use syn::parse::Parser;
use syn_grammar::grammar;
use syn_grammar::testing::Testable;

#[derive(Debug, Clone, PartialEq)]
pub enum Pat {
    Lit(String),
    Call {
        name: String,
        generics: Vec<String>,
        args: Vec<Arg>,
    },
    Bind(String, Box<Pat>),
    Group(Vec<Alt>),
    Paren(Vec<Alt>),
    Bracket(Vec<Alt>),
    Brace(Vec<Alt>),
    Opt(Box<Pat>),
    Star(Box<Pat>),
    Plus(Box<Pat>),
    Span(Box<Pat>, String),
    Cut,
}
#[derive(Debug, Clone, PartialEq)]
pub enum Arg {
    Positional(Pat),
    Named(String, Pat),
}
#[derive(Debug, Clone, PartialEq)]
pub struct Alt {
    pub seq: Vec<Pat>,
    pub label: Option<String>,
    pub action: Option<String>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct RuleDef {
    pub is_pub: bool,
    pub name: String,
    pub ret: Option<String>,
    pub alts: Vec<Alt>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Grammar {
    pub name: String,
    pub rules: Vec<RuleDef>,
}

fn call(name: &str) -> Pat {
    Pat::Call {
        name: name.into(),
        generics: vec![],
        args: vec![],
    }
}

grammar! {
    grammar Dsl {
        use quote::ToTokens;

        pub grammar_def -> Grammar =
            "grammar" name:ident { rules:rule_def* } -> {
                Grammar { name: name.to_string(), rules }
            }

        rule_def -> RuleDef =
            _a:outer_attrs vis:"pub"? "rule" name:ident ret:ret_type? "=" alts:alternatives -> {
                RuleDef {
                    is_pub: vis.is_some(),
                    name: name.to_string(),
                    ret: ret.map(|t| t.to_token_stream().to_string()),
                    alts,
                }
            }

        ret_type -> syn::Type = "->" t:rust_type # "return type" -> { t }

        alternatives -> Vec<Alt> =
            alts:separated(alternative, "|", min = 1, item_label = "alternative") -> { alts }

        alternative -> Alt =
            seq:pattern+ label:label? action:action? -> {
                Alt { seq, label, action: action.map(|b| b.to_token_stream().to_string()) }
            }

        label -> String = "#" s:string -> { s.value }
        action -> syn::Block = "->" b:rust_block -> { b }

        pattern -> Pat =
            p:postfix "@" s:ident -> { Pat::Span(Box::new(p), s.to_string()) }
          | p:postfix -> { p }

        postfix -> Pat =
            p:base "?" -> { Pat::Opt(Box::new(p)) }
          | p:base "*" -> { Pat::Star(Box::new(p)) }
          | p:base "+" -> { Pat::Plus(Box::new(p)) }
          | p:base -> { p }

        base -> Pat =
            "=>" -> { Pat::Cut }
          | s:string -> { Pat::Lit(s.value) }
          | "paren" paren(alts:alternatives) -> { Pat::Paren(alts) }
          | [ alts:alternatives ] -> { Pat::Bracket(alts) }
          | { alts:alternatives } -> { Pat::Brace(alts) }
          | paren(alts:alternatives) -> { Pat::Group(alts) }
          | not("rule") n:ident ":" p:postfix -> { Pat::Bind(n.to_string(), Box::new(p)) }
          | not("rule") n:ident g:generic_args? a:call_args? -> {
                Pat::Call {
                    name: n.to_string(),
                    generics: g.unwrap_or_default(),
                    args: a.unwrap_or_default(),
                }
            }

        generic_args -> Vec<String> =
            "<" tys:separated(rust_type, ",", min = 1) ">" -> {
                tys.iter().map(|t| t.to_token_stream().to_string()).collect()
            }

        call_args -> Vec<Arg> =
            paren(args:separated(arg, ",", item_label = "argument")) -> { args }

        // Named arguments take a pattern, a number (`min = 1`) or a flag
        // (`trailing = true`).
        arg -> Arg =
            n:ident "=" v:lit_int -> { Arg::Named(n.to_string(), Pat::Lit(v.to_string())) }
          | n:ident "=" b:bool -> { Arg::Named(n.to_string(), Pat::Lit(b.to_string())) }
          | n:ident "=" p:pattern -> { Arg::Named(n.to_string(), p) }
          | p:pattern -> { Arg::Positional(p) }
    }
}

/// The calculator from the README, verbatim.
const CALC: &str = r#"
grammar Calc {
    pub rule expression -> i32 =
        l:expression "+" r:term -> { l + r }
      | l:expression "-" r:term -> { l - r }
      | t:term                  -> { t }

    rule term -> i32 =
        f:factor "*" t:term -> { f * t }
      | f:factor            -> { f }

    rule factor -> i32 =
        i:i32               -> { i }
      | paren(e:expression) -> { e }
}
"#;

/// One of everything the DSL offers on the pattern level.
const FEATURES: &str = r##"
grammar Features {
    /// doc comment on a rule
    pub rule stmt -> Stmt =
        "let" => name:ident "=" e:expr ";" # "let statement" -> { Stmt::Let(name, e) }
      | e:expr ";" # "expression statement" -> { Stmt::Expr(e) }

    rule list -> Vec<i32> = items:separated(i32, ",", min = 1, trailing = true) -> { items }
    rule generic -> Vec<i32> = l:list<i32>(item = i32) -> { l }
    rule guarded -> () = peek("a") not("b") ("a" | "c")+ x:i32 @ sp -> { () }
    rule raw -> proc_macro2::TokenStream = "#" body:until(";") ";" -> { body }
    rule arr -> Vec<i32> = [ xs:i32* ] { ys:i32? } -> { xs }
}
"##;

#[test]
fn parses_the_readme_calculator() {
    let g = Dsl::parse_grammar_def
        .parse_str(CALC)
        .test()
        .assert_success();
    assert_eq!(g.name, "Calc");
    assert_eq!(
        g.rules.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(),
        ["expression", "term", "factor"]
    );
    assert!(g.rules[0].is_pub && !g.rules[1].is_pub);
    assert_eq!(g.rules[0].ret.as_deref(), Some("i32"));
    assert_eq!(g.rules[0].alts.len(), 3);
    assert_eq!(
        g.rules[0].alts[0].seq,
        vec![
            Pat::Bind("l".into(), Box::new(call("expression"))),
            Pat::Lit("+".into()),
            Pat::Bind("r".into(), Box::new(call("term"))),
        ]
    );
    assert_eq!(g.rules[0].alts[0].action.as_deref(), Some("{ l + r }"));
    assert!(matches!(g.rules[2].alts[1].seq[0], Pat::Paren(_)));
}

#[test]
fn parses_every_pattern_kind() {
    let g = Dsl::parse_grammar_def
        .parse_str(FEATURES)
        .test()
        .assert_success();
    let stmt = &g.rules[0];
    assert_eq!(stmt.alts[0].label.as_deref(), Some("let statement"));
    assert_eq!(stmt.alts[0].seq[1], Pat::Cut);

    let list = &g.rules[1];
    let Pat::Bind(_, inner) = &list.alts[0].seq[0] else {
        panic!("{:?}", list.alts[0].seq[0])
    };
    let Pat::Call { name, args, .. } = &**inner else {
        panic!("{inner:?}")
    };
    assert_eq!(name, "separated");
    assert_eq!(
        args,
        &[
            Arg::Positional(call("i32")),
            Arg::Positional(Pat::Lit(",".into())),
            Arg::Named("min".into(), Pat::Lit("1".into())),
            Arg::Named("trailing".into(), Pat::Lit("true".into())),
        ]
    );

    let Pat::Bind(_, inner) = &g.rules[2].alts[0].seq[0] else {
        panic!()
    };
    let Pat::Call { generics, args, .. } = &**inner else {
        panic!()
    };
    assert_eq!(generics, &["i32"]);
    assert_eq!(args, &[Arg::Named("item".into(), call("i32"))]);

    let guarded = &g.rules[3].alts[0].seq;
    assert!(matches!(&guarded[0], Pat::Call { name, .. } if name == "peek"));
    assert!(matches!(&guarded[1], Pat::Call { name, .. } if name == "not"));
    assert!(matches!(&guarded[2], Pat::Plus(inner) if matches!(**inner, Pat::Group(_))));
    assert!(matches!(&guarded[3], Pat::Span(..)));

    assert!(matches!(g.rules[5].alts[0].seq[0], Pat::Bracket(_)));
    assert!(matches!(g.rules[5].alts[0].seq[1], Pat::Brace(_)));
}

/// The messages a grammar author gets for a broken grammar - from the
/// self-hosted parser, i.e. from the diagnostics engine itself.
#[test]
fn reports_broken_grammars_with_context() {
    // An empty alternative: every kind of pattern start is listed.
    Dsl::parse_grammar_def
        .parse_str("grammar G { rule a -> i32 = x:ident \"=\" -> { 1 } | -> { 2 } }")
        .test()
        .assert_failure_contains("expected one of: `=>`")
        .assert_failure_contains("`identifier`")
        .assert_failure_contains("`string literal`")
        .assert_failure_contains("`square brackets`")
        .assert_failure_contains("; found unexpected token `-`")
        .assert_failure_contains(
            "\nin alternative 2\nin alternatives\nin rule def\nin grammar def",
        );

    // A missing argument is reported as the missing list item, at the group end.
    Dsl::parse_grammar_def
        .parse_str("grammar G { rule a = separated(ident, ) }")
        .test()
        .assert_failure_contains("unexpected end of group, expected argument")
        .assert_failure_contains("\nin argument 2\nin call args");

    // The `->` was consumed, so the type's own error wins over the label
    // `# "return type"` (ADR 13, point 7): syn's list of what can start a
    // type, placed in the grammar's rule context.
    Dsl::parse_grammar_def
        .parse_str("grammar G { rule a -> = x -> { 1 } }")
        .test()
        .assert_failure_contains("expected one of: `for`, parentheses, `fn`")
        .assert_failure_contains(
            " at column 22 (line 1)\nin ret type\nin rule def\nin grammar def",
        );

    // A rule without a name.
    Dsl::parse_grammar_def
        .parse_str("grammar G { rule a = x -> { 1 } rule = y }")
        .test()
        .assert_failure_contains("expected identifier")
        .assert_failure_contains("\nin rule def\nin grammar def");
}
