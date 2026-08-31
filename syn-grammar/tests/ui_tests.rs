#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/paren_open.rs");
    t.compile_fail("tests/ui/paren_close.rs");
    t.compile_fail("tests/ui/bracket_open.rs");
    t.compile_fail("tests/ui/bracket_close.rs");
    t.compile_fail("tests/ui/brace_open.rs");
    t.compile_fail("tests/ui/brace_close.rs");
    t.compile_fail("tests/ui/not_operator.rs");
    t.compile_fail("tests/ui/tilde_operator.rs");
    t.compile_fail("tests/ui/ampersand_operator.rs");
    t.compile_fail("tests/ui/undefined_rule_with_use.rs");
    t.compile_fail("tests/ui/syn_type_without_parse.rs");
}
