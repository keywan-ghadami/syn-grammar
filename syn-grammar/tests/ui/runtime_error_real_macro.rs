// Fehlermeldung auf dem ECHTEN Prozedurmakro-Pfad.
//
// Die uebrige Testsuite laeuft ueber `Parser::parse_str` und damit ueber den
// proc-macro2-Fallback. Hier laeuft ein wirkliches Makro - der einzige Pfad, auf
// dem sich das Verhalten im Produkteinsatz pruefen laesst. ADR 13, Punkt 14.
//
// Der Schnappschuss zeigt, dass die Meldung dort Positionen traegt. Das ist erst
// ab Rust 1.88 so (proc-macro2 setzt `proc_macro_span_location` dann auch auf
// stable); darunter waeren alle Spans (0,0). Das Projekt verlangt 1.88, dieser
// Test haelt die Zusage fest.
ui_macro::zuweisung!(let x = ;);

fn main() {}
