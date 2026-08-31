// `::` ist ein zusammenhaengender Operator. `a : : b` mit Zwischenraum darf
// NICHT passen.
//
// Der Codegen zerlegt mehrzeichige Operatoren heute pro Zeichen und prueft die
// Zusammengehoerigkeit ueber `Span::end() != Span::start()`. Im echten Makro
// sind beide `(0,0)`, die Pruefung ist dort also wirkungslos - dieser Fall
// belegt das. Ueber `parse_str` wird dieselbe Eingabe korrekt abgelehnt.
ui_macro::pfad!(a : : b);

fn main() {}
