use crate::{analysis, parser};
use proc_macro2::{Ident, TokenStream};
use syn::spanned::Spanned;

/// Was ein Backend an eingebauten Regeln mitbringt.
pub mod backend;
/// Backend-unabhaengige Werttypen fuer Aktionsbloecke.
pub mod types;

/// Eine ganze Grammatik, so wie `grammar! { … }` sie meint.
#[derive(Debug, Clone)]
pub struct GrammarDefinition {
    /// Der Name aus `grammar Name { … }`; zugleich der Name des erzeugten Moduls.
    pub name: Ident,
    /// Die erzeugten Regeln, in Quellreihenfolge.
    pub rules: Vec<Rule>,
    /// Regeln aus `extern rule …;` - sie werden **nicht** erzeugt, sondern von
    /// Hand geschrieben und nur aufgerufen.
    pub extern_rules: Vec<ExternRule>,
    /// Grammatiken aus `import … as alias;`, deren Regeln als `alias::regel`
    /// aufrufbar sind.
    pub imports: Vec<ImportedGrammar>,
    /// `use`-Anweisungen, die unveraendert in das erzeugte Modul uebernommen
    /// werden. Ein Glob darunter schaltet die "Undefined rule"-Pruefung ab -
    /// dann ist die Menge der sichtbaren Namen zur Makro-Zeit nicht bekannt.
    pub uses: Vec<syn::ItemUse>,
}

/// Eine von Hand geschriebene Regel, die die Grammatik nur aufruft.
///
/// Aus `extern rule name(p: T) -> R;`. Die Funktion muss am Definitionsort der
/// Grammatik sichtbar sein und
/// `fn name<'a>(input: &Strom<'a>, p: T) -> StreamResult<'a, R>` heissen.
#[derive(Debug, Clone)]
pub struct ExternRule {
    /// Name der Regel und damit der aufzurufenden Funktion.
    pub name: Ident,
    /// Generische Parameter der Deklaration.
    pub generics: syn::Generics,
    /// Deklarierte Parameter; sie werden hinter `input` durchgereicht.
    pub params: Vec<RuleParameter>,
    /// Der Typ, den die Funktion liefert.
    pub return_type: syn::Type,
    /// Attribute an der Deklaration.
    pub attrs: Vec<syn::Attribute>,
}

/// Eine per `import … as …;` eingebundene fremde Grammatik.
#[derive(Debug, Clone)]
pub struct ImportedGrammar {
    /// Pfad zum Modul der fremden Grammatik.
    pub path: syn::Path,
    /// Der Aliasname, unter dem ihre Regeln aufgerufen werden.
    pub alias: Ident,
}

/// Eine Regel der Grammatik.
#[derive(Debug, Clone)]
pub struct Rule {
    /// Der Regelname. Daraus werden `parse_<name>` und `parse_<name>_impl`.
    pub name: Ident,
    /// Generische Parameter, etwa `list<T>`.
    pub generics: syn::Generics,
    /// Laufzeitparameter, etwa `value(offset: i32)`.
    pub params: Vec<RuleParameter>,
    /// Der Typ hinter `->`.
    pub return_type: syn::Type,
    /// Vorab bestimmte Klassifikation von [`Rule::return_type`].
    pub return_type_kind: analysis::ReturnTypeKind,
    /// Die durch `|` getrennten Alternativen, in Quellreihenfolge.
    pub variants: Vec<RuleVariant>,
    /// Steht `pub` davor? Die Regel `main` gilt immer als oeffentlich.
    pub is_pub: bool,
    /// Lexikalische Regel: innerhalb gilt Zeichen-, nicht Tokengenauigkeit.
    pub is_lexical: bool,
    /// Attribute an der Regel; `#[doc]`, `#[cfg]` und die Lint-Attribute
    /// wandern in den erzeugten Code.
    pub attrs: Vec<syn::Attribute>,
}

/// Ein Parameter einer Regel.
#[derive(Debug, Clone)]
pub struct RuleParameter {
    /// Der Name, unter dem er im Rumpf sichtbar ist.
    pub name: Ident,
    /// Der Typ. `None` heisst: ein **Parser**-Parameter (etwa `list<T>(item)`),
    /// kein Wert - er wird zur Makro-Zeit eingesetzt, nicht zur Laufzeit
    /// uebergeben.
    pub ty: Option<syn::Type>,
}

/// Eine Alternative einer Regel.
#[derive(Debug, Clone)]
pub struct RuleVariant {
    /// Die Mustersequenz, die passen muss.
    pub pattern: Vec<ModelPattern>,
    /// Der Aktionsblock hinter `->`, der den Wert baut.
    pub action: TokenStream,
    /// Der Klartextname aus `# "…"`. Scheitert die Alternative an ihrer
    /// Anfangsstelle, tritt er als Erwartung an die Stelle der internen Meldung.
    pub label: Option<String>,
    /// Soll die Spanne der Alternative mitgeliefert werden?
    pub with_span: bool,
    /// Wurde der Aktionsblock ausgeschrieben, oder hat der Generator ihn
    /// ergaenzt? Nur Ersteres darf eine Typpruefung ausloesen.
    pub is_explicit: bool,
}

/// Ein Muster der Grammatik-DSL.
///
/// Jede Variante entspricht einer Schreibweise in `SYNTAX.md`.
#[derive(Debug, Clone)]
pub enum ModelPattern {
    /// `=>` - Cut. Ab hier ist die Ableitung festgelegt; ein Fehler dahinter
    /// ist fatal und laesst keine andere Alternative mehr zu.
    Cut(proc_macro2::Span),
    /// Ein Literalterminal, etwa `"fn"` oder `"::"`.
    Lit {
        /// Bindungsname aus `name:"fn"`.
        binding: Option<Ident>,
        /// Das Literal selbst.
        lit: syn::Lit,
    },
    /// Der Aufruf einer Regel, eines Builtins oder eines `syn::`-Typs.
    RuleCall {
        /// Bindungsname aus `name:regel`.
        binding: Option<Ident>,
        /// Der Pfad: `regel`, `alias::regel` oder `syn::Type`.
        rule_path: syn::Path,
        /// Generische Argumente, etwa `list<Vec<T>>`.
        generics: Vec<syn::Type>,
        /// Argumente in Klammern, etwa `separated(item, ",")`.
        args: Vec<Argument>,
    },
    /// Eine geklammerte Alternativenmenge, `( a | b )`.
    Group {
        /// Bindungsname der ganzen Gruppe.
        binding: Option<Ident>,
        /// Die Alternativen als Tripel aus Mustern, Aktionsblock und Label.
        alts: Vec<(Vec<ModelPattern>, Option<TokenStream>, Option<String>)>,
        /// Quellstelle der Gruppe.
        span: proc_macro2::Span,
    },
    /// `bracket( … )` - Inhalt echter eckiger Klammern.
    Bracketed(Vec<ModelPattern>, proc_macro2::Span),
    /// `brace( … )` bzw. `{ … }` - Inhalt echter geschweifter Klammern.
    Braced(Vec<ModelPattern>, proc_macro2::Span),
    /// `paren( … )` - Inhalt echter runder Klammern.
    Parenthesized(Vec<ModelPattern>, proc_macro2::Span),
    /// `x?` - hoechstens einmal.
    Optional(Box<ModelPattern>, proc_macro2::Span),
    /// `x*` - beliebig oft, auch nullmal.
    Repeat(Box<ModelPattern>, proc_macro2::Span),
    /// `x+` - mindestens einmal.
    Plus(Box<ModelPattern>, proc_macro2::Span),
    /// `x @ name` - bindet die Spanne des Musters an `name`.
    SpanBinding(Box<ModelPattern>, Ident, proc_macro2::Span),
    /// `recover(body, sync)` - scheitert `body`, wird bis `sync` uebersprungen,
    /// statt die ganze Regel scheitern zu lassen.
    Recover {
        /// Bindungsname; das Ergebnis ist ein `Option`.
        binding: Option<Ident>,
        /// Das eigentliche Muster.
        body: Box<ModelPattern>,
        /// Die Synchronisationsmarke, bis zu der uebersprungen wird.
        sync: Box<ModelPattern>,
        /// Quellstelle.
        span: proc_macro2::Span,
    },
    /// `peek(x)` - prueft, ohne zu verbrauchen.
    Peek(Box<ModelPattern>, proc_macro2::Span),
    /// `not(x)` - schlaegt fehl, wenn `x` passen wuerde. Verbraucht nichts.
    Not(Box<ModelPattern>, proc_macro2::Span),
    /// `until(x)` - sammelt rohe Tokens bis `x` passen wuerde.
    Until {
        /// Bindungsname; das Ergebnis ist ein `TokenStream`.
        binding: Option<Ident>,
        /// Das Abbruchmuster. Es wird nicht verbraucht.
        pattern: Box<ModelPattern>,
        /// Quellstelle.
        span: proc_macro2::Span,
    },
    /// `count(x)` - zaehlt, wie oft das **Element** passte.
    Count {
        /// Bindungsname; das Ergebnis ist ein `usize`.
        binding: Option<Ident>,
        /// Das gezaehlte Muster, samt seinem Wiederholungsoperator.
        pattern: Box<ModelPattern>,
        /// Quellstelle.
        span: proc_macro2::Span,
    },
    /// `lex( … )` - zeichengenaue Betrachtung innerhalb des Bereichs.
    LexicalScope(Box<ModelPattern>, proc_macro2::Span),
    /// `spaced( … )` - Leerraum zwischen Tokens wird bedeutsam.
    SpacedScope(Box<ModelPattern>, proc_macro2::Span),
    /// `fail("…")` - bricht mit dieser Meldung ab, wortwoertlich und hochprior.
    Fail {
        /// Die Meldung. Ohne sie steht dort "Explicit failure".
        message: Option<syn::Lit>,
        /// Quellstelle.
        span: proc_macro2::Span,
    },
}

impl ModelPattern {
    /// Die Quellstelle des Musters - fuer Fehlermeldungen des Generators.
    pub fn span(&self) -> proc_macro2::Span {
        match self {
            ModelPattern::Cut(s) => *s,
            ModelPattern::Lit { lit, .. } => lit.span(),
            ModelPattern::RuleCall { rule_path, .. } => {
                use syn::spanned::Spanned;
                rule_path.span()
            }
            ModelPattern::Group { span, .. } => *span,
            ModelPattern::Bracketed(_, s) => *s,
            ModelPattern::Braced(_, s) => *s,
            ModelPattern::Parenthesized(_, s) => *s,
            ModelPattern::Optional(_, s) => *s,
            ModelPattern::Repeat(_, s) => *s,
            ModelPattern::Plus(_, s) => *s,
            ModelPattern::SpanBinding(_, _, s) => *s,
            ModelPattern::Recover { span, .. } => *span,
            ModelPattern::Peek(_, s) => *s,
            ModelPattern::Not(_, s) => *s,
            ModelPattern::Until { span, .. } => *span,
            ModelPattern::Count { span, .. } => *span,
            ModelPattern::LexicalScope(_, s) => *s,
            ModelPattern::SpacedScope(_, s) => *s,
            ModelPattern::Fail { span, .. } => *span,
        }
    }
}

/// Ein Argument eines Regelaufrufs.
#[derive(Debug, Clone)]
pub enum Argument {
    /// Nach Position, etwa `separated(item, ",")`.
    Positional(ModelPattern),
    /// Nach Name, etwa `separated(item, ",", min=1)`.
    Named(Ident, ModelPattern),
}

impl From<parser::GrammarDefinition> for GrammarDefinition {
    fn from(p: parser::GrammarDefinition) -> Self {
        let mut uses = p.uses;
        if let Some(inherits) = p.inherits {
            // Deprecation warning could be emitted here if we had a way to report it
            // For now, we just map it to a use super::*; for compatibility
            let name = inherits.name;
            let item_use: syn::ItemUse = syn::parse_quote!(use super::#name::*;);
            uses.insert(0, item_use);
        }
        GrammarDefinition {
            name: p.name,
            rules: p.rules.into_iter().map(Into::into).collect(),
            extern_rules: p.extern_rules.into_iter().map(Into::into).collect(),
            imports: p.imports.into_iter().map(Into::into).collect(),
            uses,
        }
    }
}

impl From<parser::ExternRule> for ExternRule {
    fn from(p: parser::ExternRule) -> Self {
        ExternRule {
            name: p.name,
            generics: p.generics,
            params: p.params.into_iter().map(Into::into).collect(),
            return_type: p.return_type,
            attrs: p.attrs,
        }
    }
}

impl From<parser::ImportedGrammar> for ImportedGrammar {
    fn from(p: parser::ImportedGrammar) -> Self {
        ImportedGrammar {
            path: p.path,
            alias: p.alias,
        }
    }
}

impl From<parser::Rule> for Rule {
    fn from(p: parser::Rule) -> Self {
        let is_lexical = p
            .name
            .to_string()
            .chars()
            .next()
            .is_some_and(char::is_uppercase);
        let return_type_kind = analysis::determine_return_type_kind(&p.return_type);
        Rule {
            name: p.name,
            generics: p.generics,
            params: p.params.into_iter().map(Into::into).collect(),
            return_type: p.return_type,
            return_type_kind,
            variants: p.variants.into_iter().map(Into::into).collect(),
            is_pub: p.is_pub.is_some(),
            is_lexical,
            attrs: p.attrs,
        }
    }
}

impl From<parser::RuleParameter> for RuleParameter {
    fn from(p: parser::RuleParameter) -> Self {
        RuleParameter {
            name: p.name,
            ty: p.ty,
        }
    }
}

impl From<parser::RuleVariant> for RuleVariant {
    fn from(p: parser::RuleVariant) -> Self {
        RuleVariant {
            pattern: p.pattern.into_iter().map(Into::into).collect(),
            action: p.action,
            label: p.label,
            with_span: p.with_span,
            is_explicit: p.is_explicit,
        }
    }
}

impl From<parser::Pattern> for ModelPattern {
    fn from(p: parser::Pattern) -> Self {
        match p {
            parser::Pattern::Cut(token) => ModelPattern::Cut(token.span()), // FatArrow has .span()
            parser::Pattern::Lit { binding, lit } => ModelPattern::Lit { binding, lit },
            parser::Pattern::RuleCall {
                binding,
                rule_path,
                generics,
                args,
            } => ModelPattern::RuleCall {
                binding,
                rule_path,
                generics,
                args: args.into_iter().map(Into::into).collect(),
            },
            parser::Pattern::Group {
                binding,
                alts,
                token,
            } => ModelPattern::Group {
                binding,
                alts: alts
                    .into_iter()
                    .map(|(seq, action, label)| {
                        (seq.into_iter().map(Into::into).collect(), action, label)
                    })
                    .collect(),
                span: token.span.join(), // Paren has .span: DelimSpan which has .join() -> Span
            },
            parser::Pattern::Bracketed {
                binding: _,
                patterns,
                token,
            } => {
                // Ignoring binding for Bracketed as not supported in ModelPattern yet
                ModelPattern::Bracketed(
                    patterns.into_iter().map(Into::into).collect(),
                    token.span.join(),
                )
            }
            parser::Pattern::Braced {
                binding: _,
                patterns,
                token,
            } => {
                // Ignoring binding for Braced as not supported in ModelPattern yet
                ModelPattern::Braced(
                    patterns.into_iter().map(Into::into).collect(),
                    token.span.join(),
                )
            }
            parser::Pattern::Parenthesized {
                binding: _,
                patterns,
                kw_token: _,
                token,
            } => {
                // Ignoring binding for Parenthesized as not supported in ModelPattern yet
                ModelPattern::Parenthesized(
                    patterns.into_iter().map(Into::into).collect(),
                    token.span.join(),
                )
            }
            parser::Pattern::Optional(p, token) => {
                ModelPattern::Optional(Box::new((*p).into()), token.span)
            } // Question has .span (field)
            parser::Pattern::Repeat(p, token) => {
                ModelPattern::Repeat(Box::new((*p).into()), token.span)
            } // Star has .span (field)
            parser::Pattern::Plus(p, token) => {
                ModelPattern::Plus(Box::new((*p).into()), token.span)
            } // Plus has .span (field)
            parser::Pattern::SpanBinding(p, id, token) => {
                ModelPattern::SpanBinding(Box::new((*p).into()), id, token.span)
                // At has .span (field)
            }
            parser::Pattern::Recover {
                binding,
                body,
                sync,
                kw_token,
            } => ModelPattern::Recover {
                binding,
                body: Box::new((*body).into()),
                sync: Box::new((*sync).into()),
                span: kw_token.span(), // Custom Keyword has .span()
            },
            parser::Pattern::Peek(p, token) => {
                ModelPattern::Peek(Box::new((*p).into()), token.span())
            } // Custom Keyword has .span()
            parser::Pattern::Not(p, token) => {
                ModelPattern::Not(Box::new((*p).into()), token.span())
            } // Custom Keyword has .span()
            parser::Pattern::Until {
                binding,
                pattern,
                kw_token,
            } => ModelPattern::Until {
                binding,
                pattern: Box::new((*pattern).into()),
                span: kw_token.span(), // Custom Keyword has .span()
            },
            parser::Pattern::Count {
                binding,
                pattern,
                kw_token,
            } => ModelPattern::Count {
                binding,
                pattern: Box::new((*pattern).into()),
                span: kw_token.span(),
            },
            parser::Pattern::LexicalScope(pattern, kw_token) => {
                ModelPattern::LexicalScope(Box::new((*pattern).into()), kw_token.span())
            }
            parser::Pattern::SpacedScope(pattern, kw_token) => {
                ModelPattern::SpacedScope(Box::new((*pattern).into()), kw_token.span())
            }
            parser::Pattern::Fail { message, kw_token } => ModelPattern::Fail {
                message,
                span: kw_token.span(),
            },
        }
    }
}

impl From<parser::Argument> for Argument {
    fn from(p: parser::Argument) -> Self {
        match p {
            parser::Argument::Positional(p) => Argument::Positional(p.into()),
            parser::Argument::Named(n, p) => Argument::Named(n, p.into()),
        }
    }
}
