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
    t.compile_fail("tests/ui/inheritance_removed.rs");
    t.compile_fail("tests/ui/import_injection.rs");
    t.compile_fail("tests/ui/unknown_list_argument.rs");

    // Cases on the REAL procedural-macro path (crate `tests/ui-macro`).
    // Only here does a real macro run; everything else goes through `parse_str`
    // and thus the proc-macro2 fallback. What happens in production use is
    // visible exclusively here.
    t.compile_fail("tests/ui/runtime_error_real_macro.rs");
    t.pass("tests/ui/runtime_ok_real_macro.rs");
    t.compile_fail("tests/ui/joint_operator_real_macro.rs");
}
