// `::` ist ein zusammenhaengender Operator. `a : : b` mit Zwischenraum darf
// NICHT passen - auf jeder Toolchain.
//
// Frueher zerlegte der Codegen mehrzeichige Operatoren pro Zeichen und pruefte
// die Zusammengehoerigkeit ueber `Span::end() != Span::start()`. Das haengt
// daran, dass Spans im Prozedurmakro ueberhaupt Positionen tragen - was erst ab
// Rust 1.88 der Fall ist (proc-macro2 `build.rs`, cfg
// `proc_macro_span_location`). Auf aelteren Toolchains ging `a : : b` als `::`
// durch. Seit der Abbildung auf `Token![::]` prueft syn selbst `Spacing::Joint`,
// unabhaengig von der Toolchain-Version.
ui_macro::pfad!(a : : b);

fn main() {}
