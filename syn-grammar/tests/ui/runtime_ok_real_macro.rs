// Der Erfolgsfall auf dem echten Makro-Pfad. Bisher gab es keinen einzigen
// `t.pass(...)`-Fall - dass eine Grammatik im echten Makro ueberhaupt sauber
// durchlaeuft, war ungeprueft.
fn main() {
    let wert: i32 = ui_macro::zuweisung!(let x = 42;);
    assert_eq!(wert, 42);
}
