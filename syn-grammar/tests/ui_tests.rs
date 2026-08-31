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

    // Faelle auf dem ECHTEN Prozedurmakro-Pfad (Crate `tests/ui-macro`).
    // Nur hier sind die Spans `(0,0)` wie in einem realen Makro auf stable;
    // alles andere laeuft ueber `parse_str` und damit ueber den Fallback.
    t.compile_fail("tests/ui/runtime_error_real_macro.rs");
    t.pass("tests/ui/runtime_ok_real_macro.rs");
    t.compile_fail("tests/ui/joint_operator_real_macro.rs");
}
