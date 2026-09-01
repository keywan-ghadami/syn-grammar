# ADR 15: Der Weg zu linearem Parsen

**Status:** Proposed. Stufen 0 und 1 sind umgesetzt, Stufen 2–4 sind zu entscheiden.
**Datum:** 2026-08-31

## Context

Der erzeugte Parser arbeitet auf `syn::buffer::Cursor`. Für echte syn-AST-Typen
(`syn::Type`, `Generics`, `ReturnType`, `Macro`, `Block`, `Visibility`) gibt es
keinen Weg vom `Cursor` zu einem `ParseStream`, also materialisiert
`invoke_parser_fn` (`core/grammar-kit/src/combinators.rs`) pro Aufruf den
Reststrom und lässt `Parser::parse2` daraus einen **neuen `TokenBuffer`** bauen.

Bei einem AST-Typ je Listenelement ergibt das quadratisches Verhalten in der
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

### Stufe 1 — an der Delimiter-Gruppe begrenzen (**umgesetzt**)

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

**Was Stufe 1 nicht löst:** `syn::Type`, `Generics`, `ReturnType`, `Visibility`.
Die haben keine Gruppe als Grenze. Genau sie stehen im cxx-Benchmark in jedem
Funktionsargument, und dort bleibt das quadratische Verhalten.

### Stufe 2 — Winkelklammer-Fenster für `syn::Type`

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
Scanner.

### Stufe 3 — ParseStream-first (der eigentliche Fix)

Der Rumpf arbeitet auf `ParseStream` statt `Cursor`; Cursor-Primitiven laufen in
kurzen `step`-Episoden. Ein `syn::Type` kostet dann `input.parse::<T>()` —
O(Länge des Typs) statt O(Rest).

**Die Fehlerauswahl bleibt unverändert.** Das war die größte Sorge und sie löst
sich auf: `ParseBuffer::cursor()` ist öffentlich und liefert `Cursor<'a>`. Damit
funktioniert `ParseError.at` samt `progress_cmp` über `PartialOrd` genau wie
heute — auch für Cursor aus `fork()`, `parenthesized!` und
`parse_any_delimiter`, denn alle zeigen in denselben `TokenBuffer`.

Kosten: Backtracking läuft über `fork()` + `advance_to`. Ein `fork()` ist eine
kleine Rc-Allokation plus ein paar Wortkopien; `advance_to` ist laut
syn-Dokumentation O(1) unabhängig vom Abstand. Wo heute ein `Cursor`-Copy
(0 Allokationen) reicht, kostet ein Alternativenversuch künftig eine Allokation.

Milderung: Alternativen, deren Zweig **rein cursorbasiert** ist (Einzeltoken,
Literale), brauchen keinen Fork und können in einer `step`-Episode mit
Cursor-Copies backtracken. Der Codegen unterscheidet diese Fälle bereits — er
entscheidet heute zwischen `take_single`, `take_fixed` und `invoke_parser_fn`.

Das ist die einzige Variante, bei der der `TokenBuffer` wirklich **genau einmal**
gebaut wird.

### Stufe 4 — Upstream

`StepCursor::advance_to` bei syn anfragen. Der Quelltext erklärt selbst, dass es
sound wäre. Damit wäre Cursor-first ohne jeden Umweg möglich und dieser ganze
ADR gegenstandslos.

## Empfehlung

Stufe 1 ist erledigt und hat mehr gebracht als gedacht. Als Nächstes **Stufe 3**,
nicht Stufe 2: Stufe 2
löst nur `syn::Type` und hinterlässt einen selbstgebauten Scanner, den niemand
mehr anfassen will; Stufe 3 löst das Problem an der Wurzel und für alle Typen.

Stufe 3 ist allerdings ein Umbau des Codegens und gehört als eigenes Vorhaben
geplant — mit derselben Messung als Abnahme. Der Umbau ist die Umkehrung dessen,
was im Mai 2026 geschah; die damalige Begründung (Backtracking wird trivial) ist
weiterhin richtig und muss durch die selektive Fork-Strategie erhalten bleiben.

Stufe 4 parallel versuchen — sie kostet fast nichts und macht im Erfolgsfall
Stufe 3 überflüssig.

## Consequences

* Die Messung ist Teil der Abnahme jeder Stufe. Reproduzierbar über zwei
  Grammatiken, die sich nur im Argumenttyp unterscheiden.
* Bis Stufe 3 bleibt die Zusage: linear in allem außer der Zahl der
  AST-Typ-Vorkommen je Delimiter-Gruppe.
* Wer eine Grammatik mit vielen `syn::Type` in einer langen Liste schreibt,
  sollte das bis dahin wissen. Gehört nach `docs/LIMITATIONS.md`.
