// Fehlermeldung auf dem ECHTEN Prozedurmakro-Pfad.
//
// Die uebrige Testsuite laeuft ueber `Parser::parse_str` und damit ueber den
// proc-macro2-Fallback, der echte Zeilen und Spalten hat. Hier laeuft ein
// wirkliches Makro: dort liefert `Span::start()` auf stable fuer jeden Span
// `(0,0)`. Der .stderr-Schnappschuss haelt fest, wie eine Meldung unter dieser
// Bedingung aussieht - ohne Positionsangabe, aber mit vollstaendiger Erwartung
// und Regelstapel. ADR 13, Punkt 14.
ui_macro::zuweisung!(let x = ;);

fn main() {}
