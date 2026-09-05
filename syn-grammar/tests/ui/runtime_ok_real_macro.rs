// The success case on the real macro path. Until now there was not a single
// `t.pass(...)` case - that a grammar runs through cleanly in the real macro
// at all was unchecked.
fn main() {
    let wert: i32 = ui_macro::zuweisung!(let x = 42;);
    assert_eq!(wert, 42);
}
