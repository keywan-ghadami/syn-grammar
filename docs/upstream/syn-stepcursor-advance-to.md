# Upstream-Anfrage an `syn`: `StepCursor::advance_to`

**Status:** Entwurf. Noch **nicht** eingereicht — das geht an ein fremdes Projekt
und braucht die Freigabe des Eigentuemers dieses Repos.
**Bezug:** ADR 15, Stufe 4.

## Worum es geht

`syn::parse::advance_step_cursor` ist `pub(crate)`:

```rust
pub(crate) fn advance_step_cursor<'c, 'a>(proof: StepCursor<'c, 'a>, to: Cursor<'c>) -> Cursor<'a>
```

Der Quelltext von `ParseBuffer::step` sagt selbst, dass eine oeffentliche
Fassung sicher waere (syn 2.0.117, `src/parse.rs`):

> In some cases it may be necessary for `R` to contain a `Cursor<'a>`. Within
> Syn we solve this using `advance_step_cursor` which uses the existence of a
> `StepCursor<'c, 'a>` as proof that it is safe to cast from `Cursor<'c>` to
> `Cursor<'a>`. **If needed outside of Syn, it would be safe to expose that API
> as a method on `StepCursor`.**

Die Anfrage ist genau das: die bereits vorhandene Funktion als Methode
freischalten. Kein neues Verhalten, keine neue Invariante.

## Der Patch

Sechs Zeilen in `impl<'c, 'a> StepCursor<'c, 'a>`:

```rust
/// Converts a cursor derived from this step cursor into one carrying the
/// lifetime of the underlying parse stream.
///
/// The existence of a `StepCursor<'c, 'a>` is proof that `'c` outlives `'a`;
/// see the comments on the struct definition.
pub fn advance_to(self, to: Cursor<'c>) -> Cursor<'a> {
    advance_step_cursor(self, to)
}
```

`StepCursor` ist `Copy`, `self` per Wert ist also unproblematisch.

## Der Anwendungsfall

Ein Parsergenerator, dessen Fehlertyp einen `Cursor` traegt — bei uns fuer den
Fortschrittsvergleich zwischen konkurrierenden Fehlern (O(1) Zeigervergleich im
gemeinsamen `TokenBuffer`, unabhaengig von Span-Positionen):

```rust
pub struct ParseError<'a> {
    pub span: Span,
    pub at: Option<Cursor<'a>>,   // <- hieran haengt es
    pub message: String,
    // ...
}
```

Der Regelrumpf laeuft auf einem `ParseBuffer<'a>`, einzelne Primitiven aber auf
dem `Cursor` — sie sind dort O(1) und brauchen keinen Strom. Um so eine
Primitive auf dem Strom laufen zu lassen und ihn um genau ihr Ergebnis
vorzuruecken, ist `step` der einzige Weg. Da die Closure fuer **jede**
Lebensdauer `'c` funktionieren muss, kann ein `ParseError<'c>` sie nicht
verlassen.

Heute wird der Fehler deshalb **ohne** seinen Cursor durch die Schranke getragen
und draussen an die Eintrittsstelle neu gehaengt:

```rust
let mut merk: Option<(Span, String, u8, bool)> = None;
let ergebnis = input.step(|sc| match f(*sc) {
    Ok((wert, danach)) => Ok((wert, danach)),
    Err(e) => {
        merk = Some((e.span, e.message, e.priority, e.is_fatal));
        Err(syn::Error::new(e.span, "unreachable"))   // nur, um step das Vorruecken zu verweigern
    }
});
```

Mit der Methode entfaellt der Umweg:

```rust
let ergebnis = input.step(|sc| match f(*sc) {
    Ok((wert, danach)) => Ok((wert, danach)),
    Err(e) => {
        fehler = Some(e.mit_cursor(e.at.map(|c| sc.advance_to(c))));  // behaelt seine Stelle
        Err(syn::Error::new(e.span, "unreachable"))
    }
});
```

## Was es *nicht* bringt

Bei uns heute **nichts an beobachtbarem Verhalten**. Alle betroffenen Primitiven
melden ihren Fehler ohnehin an der Eintrittsstelle, die Rekonstruktion ist also
exakt. Die Testsuite ist mit beiden Fassungen gleich gruen.

Der Gewinn ist, dass der Umweg verschwindet und die stillschweigende Bedingung
entfaellt, dass eine Primitive nur an ihrer Eintrittsstelle scheitern darf.

Das gehoert in die Anfrage: es waere unredlich, hier Dringlichkeit zu behaupten,
die nicht besteht. Das Argument ist, dass syn die Aenderung selbst als sicher
bezeichnet und sie sechs Zeilen kostet — nicht, dass sie hier brennt.

## Nachweis

Lokal gegen syn 2.0.117 mit genau diesem Patch gebaut, `schritt`
(`core/grammar-kit/src/stream.rs`) auf die obige Form umgestellt:
**153 Tests gruen / 0 rot**, identisch zum ungepatchten Stand. Der Patch und die
Umstellung sind danach zurueckgebaut worden; im Repo steht weiterhin die Fassung
ohne die API.

## Vorgeschlagener Weg

Ein Pull Request gegen `dtolnay/syn` mit dem Patch oben, im Text auf den
bestehenden Kommentar in `step` verweisend. Ein Issue waere der schwaechere Weg:
die Aenderung ist kleiner als ihre Beschreibung.
