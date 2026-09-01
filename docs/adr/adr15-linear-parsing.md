# ADR 15: Der Weg zu linearem Parsen

**Status:** Accepted. Stufen 0, 1 und 3 sind umgesetzt; Stufe 1 ist von Stufe 3
wieder abgelöst worden. Stufe 2 entfällt. Stufe 4 ist vorbereitet, aber nicht
eingereicht — sie liegt bei einem fremden Projekt.
**Datum:** 2026-08-31, fortgeschrieben 2026-09-01

## Context

> Der Abschnitt beschreibt den Stand **vor** der Entscheidung. Seit Stufe 3
> arbeitet der Regelrumpf auf einem `ParseBuffer`; die Brücke gibt es nicht mehr.

Der erzeugte Parser arbeitete auf `syn::buffer::Cursor`. Für echte syn-AST-Typen
(`syn::Type`, `Generics`, `ReturnType`, `Macro`, `Block`, `Visibility`) gibt es
keinen Weg vom `Cursor` zu einem `ParseStream`, also materialisierte
`invoke_parser_fn` (`core/grammar-kit/src/combinators.rs`) pro Aufruf den
Reststrom und ließ `Parser::parse2` daraus einen **neuen `TokenBuffer`** bauen.

Bei einem AST-Typ je Listenelement ergab das quadratisches Verhalten in der
Länge dieser Liste.

### Gemessen

Generierte Argumentliste, zwei Grammatiken, identisch bis auf den Typ des
Arguments — `t:syn::Type` (Brücke) gegen `t:any_ident` (O(1)):

| n | mit Brücke | ohne Brücke |
|---|---|---|
| 100 | 3,36 ms | 221 µs |
| 500 | 76,99 ms | 1,31 ms |
| 2000 | **1,40 s** | **5,32 ms** |

Zwanzigfache Eingabe kostet **417×** Zeit mit Brücke und **24×** ohne. Die
Brücke ist damit nicht *ein* Faktor, sondern die **einzige** verbliebene Quelle
des quadratischen Verhaltens — alles andere ist bereits linear.

Exponent aus den Messpunkten: log(21,5)/log(5) = 1,91 und log(15,6)/log(4) =
1,98. Sauber quadratisch, wie das Modell „n Parses × O(n) Buffer-Bau" vorhersagt.

### Wo die Zeit hingeht

Nicht in `cursor.token_stream()`. Das läuft nur über die **obersten**
Token-Trees und klont Gruppen per `Rc` in O(1) (proc-macro2, `RcVec::clone`).

Sondern in `TokenBuffer::new2` innerhalb von `parse2` (`syn`, `buffer.rs`):
ein **rekursiver Tiefendurchlauf über alle verschachtelten Tokens**, der je
Token einen `Entry` konstruiert (bei Gruppen zwei, plus Offset-Backpatching),
in einem wachsenden `Vec` sammelt und am Ende in eine `Box<[Entry]>` kopiert.
Im echten Prozedurmakro kommt pro Gruppe ein Roundtrip über die rustc-Bridge
hinzu (`proc_macro2::wrapper`, `DeferredTokenStream::new`).

Bei n=2000 sind das ~555 µs je Argument bei im Mittel ~4000 Resttokens, also
~140 ns pro Token. Zu viel für reines `Rc::clone` — die Zeit steckt in
Allokation und `Entry`-Bau.

**Konsequenz: „nur die Materialisierung optimieren" bringt wenig.
`TokenBuffer::new2` muss weg oder auf die Typlänge begrenzt werden.**

## Die Randbedingung, erschöpfend geprüft

Es gibt **keinen** öffentlichen Weg `Cursor → ParseStream`, und das ist Absicht:

* `ParseBuffer` hat gar keinen Konstruktor. Es gibt nur die freie Funktion
  `pub(crate) fn new_parse_buffer` (`syn`, `parse.rs`).
* Die Begründung steht im Quelltext und ist sicherheitstechnisch:
  `ParseBuffer` hält `Cell<Cursor<'static>>` mit `PhantomData<Cursor<'a>>`;
  eine API, die einen `Cursor<'a>` annimmt und ihm die Lebensdauer glaubt, wäre
  unsound.
* `discouraged::Speculative::advance_to` braucht **zwei** existierende Buffer und
  erzeugt keinen.
* `discouraged::AnyDelimiter::parse_any_delimiter` ruft `new_parse_buffer`
  tatsächlich von außen auf — aber nur für den **Inhalt einer Delimiter-Gruppe**
  an der aktuellen Position, und es braucht bereits ein `&ParseBuffer`.
* `ParseStream::step` ist eine Einbahnstraße: `StepCursor` derefs auf `Cursor`,
  aber `advance_step_cursor` ist `pub(crate)`. Der Rückweg scheitert an der
  Invarianz von `'c` in `StepCursor<'c, 'a>`.
* Kein Feature-Flag von syn (`parsing`, `full`, `derive`, `proc-macro`, …)
  schaltet etwas davon frei.

Bemerkenswert: syn kommentiert an `step` selbst, es *wäre* sicher,
`advance_step_cursor` als Methode auf `StepCursor` anzubieten — es ist nur nicht
getan. Siehe Stufe 4.

## Entscheidung: ein Stufenplan

### Stufe 0 — `peek_syn` ohne Allokation (**umgesetzt**)

`peek_syn` baute pro Peek einen Tokenstrom **und einen kompletten `TokenBuffer`**.
Ein Fenster von n Tokens hilft dagegen nicht, denn ein einzelnes Token kann eine
beliebig große Delimiter-Gruppe sein — `cursor.token_tree()` liefert
`{ …1000 Tokens… }` als *einen* Tree.

Jetzt reine Zeigerarithmetik über `<P::Token as syn::token::Token>::peek(cursor)`
— genau das, was syns eigenes `ParseStream::peek` tut. Null Allokationen.

`Peek::Token` und `Token::peek` sind `#[doc(hidden)]`, also ohne
Semver-Versprechen; deshalb an genau einer Stelle gekapselt.

Wirkt vor allem im Recover-Sync-Scan, der in einer Schleife steht.

### Stufe 1 — an der Delimiter-Gruppe begrenzen (**umgesetzt, dann abgelöst**)

Ein `syn::Block` **ist** genau ein `{}`-Token-Tree. `cursor.group(Delimiter::Brace)`
liefert Inhalt und Folgecursor in O(1); materialisiert wird nur der Inhalt, also
genau das, was auch geparst wird. `take_braced_block` baut den `Block` daraus
zusammen — inhaltlich dasselbe wie syns `impl Parse for Block`
(`braced!` + `Block::parse_within`), nur ohne ParseStream.

Für `syn::Macro` gilt es sinngemäß: ein Makroaufruf ist `pfad ! (…)`, und der
Pfad davor besteht nur aus Bezeichnern und `::` — er kann keine Gruppe enthalten.
`take_upto_group` materialisiert deshalb bis **einschließlich der ersten Gruppe**.
Kommt keine, fällt es auf das bisherige Verhalten zurück; teurer wird es nie.

**Gemessen**, n Einträge in einer Liste:

| n | `syn::Block` vorher | nachher | `syn::Macro` vorher | nachher |
|---|---|---|---|---|
| 100 | 3,37 ms | 665 µs | 3,06 ms | 219 µs |
| 500 | 82,52 ms | 2,80 ms | 73,17 ms | 890 µs |
| 2000 | **1,06 s** | **11,46 ms** | **1,15 s** | **3,60 ms** |

Faktor 92 bzw. 319 bei n=2000, und in beiden Fällen **quadratisch → linear**
(zwanzigfache Eingabe: 315× bzw. 376× vorher, 17× bzw. 16× nachher).

Der Effekt ist größer als erwartet, weil nicht nur die Materialisierung entfällt,
sondern auch der `TokenBuffer`-Bau über den Rest — und der dominiert.

**Was Stufe 1 nicht löste:** `syn::Type`, `Generics`, `ReturnType`, `Visibility`.
Die haben keine Gruppe als Grenze. Genau sie stehen im cxx-Benchmark in jedem
Funktionsargument, und dort blieb das quadratische Verhalten.

**Nachtrag.** Stufe 3 hat `take_braced_block` und `take_upto_group` gegenstandslos
gemacht und beide sind wieder entfernt: sie umgingen die Materialisierung, die es
seither gar nicht mehr gibt. Verloren ist dadurch nichts — die Messung oben hat
den Weg zu Stufe 3 gewiesen, weil sie zeigte, dass nicht die Materialisierung
dominiert, sondern der `TokenBuffer`-Bau.

### Stufe 2 — Winkelklammer-Fenster für `syn::Type` (**entfällt**)

Der Codegen kennt die Folgemenge nicht, aber die Struktur von Typen ist
scannbar. Delimiter-Gruppen sind je **ein** Token-Tree und damit automatisch
opak; auf Tiefe 0 zählt `<` hoch und `>` herunter — außer wenn dem `>`
unmittelbar ein `Joint`-`-` vorausging (`->` in `Fn(A) -> B`).

`<<` und `>>` sind in proc-macro2 zwei getrennte `Punct`, zählen also korrekt
±2. Lifetimes (`'a`) berühren den Zähler nicht. Vergleichsoperatoren kommen auf
Typebene nicht vor.

**Sound für `syn::Type`, `ReturnType`, `Generics` — nicht für `Expr`.** Dort sind
`<` und `>` echte Operatoren; `Expr` bliebe bei der vollen Brücke, `Block` ist
über Stufe 1 abgedeckt.

Bringt O(n), lässt die Architektur intakt, kostet einen sorgfältig getesteten
Scanner. **Nicht umgesetzt und nicht mehr nötig** — Stufe 3 löst dasselbe Problem
für alle Typen und ohne eigenen Scanner.

### Stufe 3 — ParseStream-first (**umgesetzt**)

Der Rumpf arbeitet auf einem `ParseBuffer` statt auf dem `Cursor`; die
Blatt-Primitiven laufen in kurzen `step`-Episoden. Ein `syn::Type` kostet
`input.parse::<T>()`, also O(Länge des Typs) statt O(Rest). Der `TokenBuffer`
wird genau einmal gebaut.

**Die Signatur ist der Angelpunkt.** Eine Regel heißt

```rust
fn parse_x_impl<'a>(input: &Strom<'a>, ctx: &mut ParseContext<'a>) -> StreamResult<'a, T>
```

mit `Strom<'a> = ParseBuffer<'a>` — bewusst **nicht** syns Alias
`ParseStream<'a> = &'a ParseBuffer<'a>`. Der Alias setzt die Lebensdauer der
Referenz mit der der Tokens gleich; ein `input.fork()` lebt aber nur bis zum Ende
des Stapelrahmens, womit `'a` auf diesen Rahmen verkürzt würde und ein
`ParseError<'a>` aus einer Gabel den Aufruf nicht mehr verlassen könnte. Mit
`&Strom<'a>` bleibt die Referenz-Lebensdauer frei. Das war vorab am lauffähigen
Spike geprüft, nicht erschlossen.

**Die Fehlerauswahl bleibt unverändert.** `ParseBuffer::cursor()` ist öffentlich
und liefert `Cursor<'a>`; `fork()` klont nur die Cursor-Zelle, der `TokenBuffer`
bleibt derselbe. Damit greift `same_buffer` und `PartialOrd` vergleicht Cursor
aus Gabel und Elternstrom wie bisher. Am Quelltext geprüft und im Spike belegt.

**Zwei syn-APIs mussten gekapselt werden**, beide `#[doc(hidden)]` und damit ohne
Semver-Versprechen — wie schon `Token::peek` in `peek_syn`, deshalb an je einer
Stelle:

* `syn::__private::parse_{parens,braces,brackets}` für den Abstieg in eine
  Delimiter-Gruppe. Die Makros `parenthesized!` & Co. gehen nicht: ihr Fehlerpfad
  ist ein nacktes `return Err(syn::Error)`. `AnyDelimiter::parse_any_delimiter`
  geht ebenfalls nicht — seine Rückgabe ist auf die Lebensdauer von `&self`
  verkürzt, womit kein Fehler aus der Gruppe nach außen trüge.
* `ParseBuffer::step` in `schritt`, um Cursor-Primitiven auf dem Strom laufen zu
  lassen. Da `step` eine Closure für **jede** Lebensdauer verlangt, kann ein
  `ParseError<'c>` sie nicht verlassen; `schritt` trägt ihn ohne seinen Cursor
  hindurch und hängt ihn draußen an die Eintrittsstelle. Diese Primitiven melden
  ihren Fehler ohnehin dort.

**Kosten.** Wo ein Cursor-Copy genügte, kostet ein Rücksetzpunkt jetzt eine
`fork()`-Allokation. Betroffen ist jede Alternative, jedes `?`/`*`/`+`, `peek`,
`not`, `recover` und jedes Listenelement. Ein weiterer Unterschied: nach einem
Fehler ist der Strom möglicherweise vorgerückt — Zurücksetzen ist nicht mehr
gratis, sondern muss über eine Gabel laufen. Der Codegenerator tut das an jeder
Rücksetzstelle.

**Gemessen**, dasselbe Testprogramm vorher und nachher, zwei Grammatiken, die
sich nur im Argumenttyp unterscheiden, `--release`:

| n | `syn::Type` vorher | nachher | `any_ident` vorher | nachher |
|---|---|---|---|---|
| 100 | 3,30 ms | 326 µs | 230 µs | 184 µs |
| 500 | 75,80 ms | 1,43 ms | 822 µs | 819 µs |
| 2000 | **1,174 s** | **5,33 ms** | 4,01 ms | 3,11 ms |

Faktor 220 bei n=2000. Entscheidender ist die Form: zwanzigfache Eingabe kostete
vorher 356×, jetzt 16× — sauber linear. Und `syn::Type` liegt jetzt innerhalb von
1,7× von `any_ident`, das gar keinen AST-Typ enthält; der Aufwand der Brücke ist
damit nicht verkleinert, sondern verschwunden.

Die Allokationskosten des Backtrackings sind in diesen Zahlen enthalten und
gehen unter — `any_ident` wurde sogar schneller.

### Stufe 4 — Upstream (**vorbereitet, nicht eingereicht**)

`syn::parse::advance_step_cursor` ist `pub(crate)`. Der Quelltext von
`ParseBuffer::step` sagt selbst, eine oeffentliche Fassung als Methode auf
`StepCursor` waere sicher.

**Die Begruendung hat sich mit Stufe 3 geaendert.** Urspruenglich stand hier:
„damit waere Cursor-first ohne Umweg moeglich und dieser ADR gegenstandslos".
Das ist ueberholt — Cursor-first ist nicht mehr das Ziel. Was bleibt, ist
kleiner und konkreter: `schritt` muss den Fehler einer Cursor-Primitive **ohne**
seinen Cursor durch die `step`-Schranke tragen, weil die Closure fuer jede
Lebensdauer `'c` funktionieren muss und ein `ParseError<'c>` sie nicht verlassen
kann. Mit `StepCursor::advance_to` liesse sich der Cursor von `'c` auf `'a`
heben und der Fehler bliebe unangetastet.

**Gemessen am Nutzen: klein.** Alle heutigen Primitiven melden ihren Fehler an
der Eintrittsstelle, die Rekonstruktion in `schritt` ist also exakt. Lokal gegen
ein gepatchtes syn 2.0.117 gebaut und `schritt` umgestellt: 153 Tests gruen,
identisch zum ungepatchten Stand — **kein beobachtbarer Verhaltensunterschied**.
Der Gewinn ist der entfallende Umweg und die entfallende stillschweigende
Bedingung, dass eine Primitive nur an ihrer Eintrittsstelle scheitern darf.

Der Entwurf der Anfrage liegt in
[`docs/upstream/syn-stepcursor-advance-to.md`](../upstream/syn-stepcursor-advance-to.md).
Eingereicht ist er nicht — das geht an ein fremdes Projekt und braucht eine
Entscheidung des Eigentuemers.

## Empfehlung

Stufe 3 ist umgesetzt und beseitigt die letzte quadratische Quelle. Damit ist
das Ziel des ADR erreicht: der Parser ist linear in der Eingabelänge.

**Stufe 4 ist kein Leistungsthema mehr**, sondern Aufräumen an einer
Schnittstelle — sechs Zeilen bei syn, kein beobachtbarer Unterschied hier. Der
Entwurf liegt bereit; ob er eingereicht wird, ist eine Entscheidung, keine
technische Frage.

Der Umbau ist die Umkehrung dessen, was im Mai 2026 geschah. Die damalige
Begründung — Backtracking wird trivial — war richtig und ist durch die
Gabel-Strategie erhalten: der Codegenerator setzt an denselben Stellen zurück wie
zuvor, nur eben über `fork`/`advance_to` statt über einen Cursor-Copy.

## Consequences

* Die Messung ist Teil der Abnahme jeder Stufe. Reproduzierbar über zwei
  Grammatiken, die sich nur im Argumenttyp unterscheiden.
* Die Zusage lautet jetzt: linear in der Eingabelänge. Kein Parseschritt
  materialisiert mehr den Reststrom.
* Der Preis steht in `docs/LIMITATIONS.md`: jeder Rücksetzpunkt kostet eine
  `Rc`-Allokation.
* `extern`-Regeln ändern ihre Signatur — siehe CHANGELOG.
