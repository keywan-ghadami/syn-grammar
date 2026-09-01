// Moved from macros/src/parser.rs
use proc_macro2::TokenStream;
use quote::{quote, ToTokens, TokenStreamExt};
use syn::parse::{Parse, ParseStream};
use syn::{token, Attribute, Generics, Ident, ItemUse, Lit, Path, Result, Token, Type};

mod rt {
    use syn::ext::IdentExt;
    use syn::parse::discouraged::Speculative;
    use syn::parse::ParseStream;
    use syn::Result;

    pub fn attempt<T>(
        input: ParseStream,
        parser: impl FnOnce(ParseStream) -> Result<T>,
    ) -> Result<Option<T>> {
        let fork = input.fork();
        match parser(&fork) {
            Ok(res) => {
                input.advance_to(&fork);
                Ok(Some(res))
            }
            Err(_) => Ok(None),
        }
    }

    pub fn parse_ident(input: ParseStream) -> Result<syn::Ident> {
        input.call(syn::Ident::parse_any)
    }
}

/// Die Schluesselwoerter der Grammatik-DSL.
///
/// Es sind **keine** Rust-Schluesselwoerter, sondern `custom_keyword!`-Typen:
/// `grammar`, `rule` und `peek` bleiben als Bezeichner benutzbar, etwa als
/// Regelnamen. Nur `extern`, `pub` und `as` sind echte Rust-Tokens.
#[allow(missing_docs)]
pub mod kw {
    syn::custom_keyword!(grammar);
    syn::custom_keyword!(rule);
    syn::custom_keyword!(paren);
    syn::custom_keyword!(recover);
    syn::custom_keyword!(peek);
    syn::custom_keyword!(not);
    syn::custom_keyword!(until);
    syn::custom_keyword!(import);
    syn::custom_keyword!(fail);
    syn::custom_keyword!(count);
    syn::custom_keyword!(lex);
    syn::custom_keyword!(spaced);
}

fn parse_path_no_args(input: ParseStream) -> Result<Path> {
    let leading_colon = if input.peek(Token![::]) {
        Some(input.parse::<Token![::]>()?)
    } else {
        None
    };

    let mut segments = syn::punctuated::Punctuated::new();
    loop {
        let ident: Ident = rt::parse_ident(input)?;
        let arguments = syn::PathArguments::None;
        segments.push_value(syn::PathSegment { ident, arguments });

        if input.peek(Token![::]) {
            let punct = input.parse::<Token![::]>()?;
            segments.push_punct(punct);
        } else {
            break;
        }
    }

    Ok(Path {
        leading_colon,
        segments,
    })
}

#[derive(Debug, Clone)]
/// `extern rule name(p: T) -> R;` - syntaktische Form.
///
/// Semantisches Gegenstueck: [`crate::model::ExternRule`].
pub struct ExternRule {
    /// Attribute vor der Deklaration.
    pub attrs: Vec<Attribute>,
    /// Der Regelname.
    pub name: Ident,
    /// Generische Parameter.
    pub generics: Generics,
    /// Deklarierte Parameter.
    pub params: Vec<RuleParameter>,
    /// Der Typ hinter `->`.
    pub return_type: Type,
}

impl Parse for ExternRule {
    fn parse(input: ParseStream) -> Result<Self> {
        let attrs = Attribute::parse_outer(input)?;
        let _ = input.parse::<Token![extern]>()?;
        let _ = input.parse::<kw::rule>()?;
        let name = rt::parse_ident(input)?;
        let generics: Generics = input.parse()?;

        let params = if input.peek(token::Paren) {
            let content;
            syn::parenthesized!(content in input);
            let mut params = Vec::new();
            while !content.is_empty() {
                params.push(content.parse()?);
                if content.peek(Token![,]) {
                    let _ = content.parse::<Token![,]>()?;
                }
            }
            params
        } else {
            Vec::new()
        };

        let _ = input.parse::<Token![->]>()?;
        let return_type = input.parse::<Type>()?;
        let _ = input.parse::<Token![;]>()?;

        Ok(ExternRule {
            attrs,
            name,
            generics,
            params,
            return_type,
        })
    }
}

#[derive(Debug, Clone)]
/// `import pfad as alias;` - syntaktische Form.
///
/// Semantisches Gegenstueck: [`crate::model::ImportedGrammar`].
pub struct ImportedGrammar {
    /// Pfad zur fremden Grammatik.
    pub path: Path,
    /// Der Alias hinter `as`.
    pub alias: Ident,
}

impl Parse for ImportedGrammar {
    fn parse(input: ParseStream) -> Result<Self> {
        let _ = input.parse::<kw::import>()?;
        let path = input.parse::<Path>()?; // No `grammar` keyword in path parsing
        let _ = input.parse::<Token![as]>()?;
        let alias = input.parse::<Ident>()?;
        let _ = input.parse::<Token![;]>()?;
        Ok(ImportedGrammar { path, alias })
    }
}

#[derive(Debug, Clone)]
/// Der Inhalt eines `grammar! { … }` - syntaktische Form.
///
/// Semantisches Gegenstueck: [`crate::model::GrammarDefinition`], in das diese
/// Form per `Into` ueberfuehrt wird.
pub struct GrammarDefinition {
    /// Der Name aus `grammar Name { … }`.
    pub name: Ident,
    /// Ein `: Basis` hinter dem Namen, falls vorhanden.
    pub inherits: Option<InheritanceSpec>,
    /// `use`-Anweisungen, die in das erzeugte Modul uebernommen werden.
    pub uses: Vec<ItemUse>,
    /// Die Regeln in Quellreihenfolge.
    pub rules: Vec<Rule>,
    /// Die `extern rule`-Deklarationen.
    pub extern_rules: Vec<ExternRule>,
    /// Die `import`-Anweisungen; sie duerfen vor oder im Block stehen.
    pub imports: Vec<ImportedGrammar>,
}

impl Parse for GrammarDefinition {
    fn parse(input: ParseStream) -> Result<Self> {
        // Parse top-level imports that might appear before `grammar Name { ... }`
        let mut top_level_imports = Vec::new();
        while input.peek(kw::import) {
            top_level_imports.push(input.parse()?);
        }

        let _ = input.parse::<kw::grammar>()?;
        let name = rt::parse_ident(input)?;

        let inherits = if input.peek(Token![:]) {
            Some(input.parse::<InheritanceSpec>()?)
        } else {
            None
        };

        let content;
        let _ = syn::braced!(content in input);

        let mut uses = Vec::new();
        let mut rules = Vec::new();
        let mut extern_rules = Vec::new();
        let mut nested_imports = Vec::new();

        while !content.is_empty() {
            if content.peek(Token![use]) {
                uses.push(content.parse()?);
            } else if content.peek(kw::import) {
                nested_imports.push(content.parse()?);
            } else if content.peek(Token![extern]) {
                extern_rules.push(content.parse()?);
            } else {
                // Try parsing as rule (it might have attributes)
                rules.push(content.parse()?);
            }
        }

        let mut imports = top_level_imports;
        imports.extend(nested_imports);

        Ok(GrammarDefinition {
            name,
            inherits,
            uses,
            rules,
            extern_rules,
            imports,
        })
    }
}

impl ToTokens for GrammarDefinition {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let name = &self.name;
        let inherits = &self.inherits;
        let uses = &self.uses;
        let rules = &self.rules;
        // extern_rules and imports are not currently emitted in ToTokens as they are structural metadata

        tokens.append_all(quote! {
            grammar #name #inherits {
                #(#uses)*
                #(#rules)*
            }
        });
    }
}

#[derive(Debug, Clone)]
/// Ein `: Basis` hinter dem Grammatiknamen.
pub struct InheritanceSpec {
    /// Name der Basisgrammatik.
    pub name: Ident,
}

impl Parse for InheritanceSpec {
    fn parse(input: ParseStream) -> Result<Self> {
        let _ = input.parse::<Token![:]>()?;
        let name = rt::parse_ident(input)?;
        Ok(InheritanceSpec { name })
    }
}

impl ToTokens for InheritanceSpec {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let name = &self.name;
        tokens.append_all(quote! { : #name });
    }
}

#[derive(Debug, Clone)]
/// Ein Parameter einer Regel - syntaktische Form.
///
/// Semantisches Gegenstueck: [`crate::model::RuleParameter`].
pub struct RuleParameter {
    /// Der Parametername.
    pub name: Ident,
    /// Der Typ, oder `None` fuer einen Parser-Parameter (`list<T>(item)`).
    pub ty: Option<Type>,
}

impl Parse for RuleParameter {
    fn parse(input: ParseStream) -> Result<Self> {
        let name: Ident = input.parse()?;
        let ty = if input.peek(Token![:]) {
            let _ = input.parse::<Token![:]>()?;
            Some(input.parse()?)
        } else {
            None
        };
        Ok(RuleParameter { name, ty })
    }
}

impl ToTokens for RuleParameter {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let name = &self.name;
        if let Some(ty) = &self.ty {
            tokens.append_all(quote! { #name : #ty });
        } else {
            tokens.append_all(quote! { #name });
        }
    }
}

#[derive(Debug, Clone)]
/// Eine Regel - syntaktische Form.
///
/// Semantisches Gegenstueck: [`crate::model::Rule`]. Anders als dort ist die
/// Klassifikation des Rueckgabetyps hier noch nicht bestimmt.
pub struct Rule {
    /// Attribute vor der Regel.
    pub attrs: Vec<Attribute>,
    /// Das `pub`-Token, falls vorhanden.
    pub is_pub: Option<Token![pub]>,
    /// Der Regelname.
    pub name: Ident,
    /// Generische Parameter.
    pub generics: Generics,
    /// Laufzeit- und Parser-Parameter.
    pub params: Vec<RuleParameter>,
    /// Der Typ hinter `->`.
    pub return_type: Type,
    /// Die durch `|` getrennten Alternativen.
    pub variants: Vec<RuleVariant>,
}

impl Parse for Rule {
    fn parse(input: ParseStream) -> Result<Self> {
        let attrs = Attribute::parse_outer(input)?;

        let is_pub = if input.peek(Token![pub]) {
            Some(input.parse()?)
        } else {
            None
        };

        if input.peek(kw::rule) {
            let _ = input.parse::<kw::rule>()?;
        }
        let name = rt::parse_ident(input)?;

        let generics: Generics = input.parse()?;

        let params = if input.peek(token::Paren) {
            let content;
            syn::parenthesized!(content in input);
            let mut params = Vec::new();
            while !content.is_empty() {
                params.push(content.parse()?);
                if content.peek(Token![,]) {
                    let _ = content.parse::<Token![,]>()?;
                }
            }
            params
        } else {
            Vec::new()
        };

        let return_type = if input.peek(Token![->]) {
            let _ = input.parse::<Token![->]>()?;
            input.parse::<Type>()?
        } else {
            syn::parse_quote!(())
        };

        let capture_span = if input.peek(Token![@]) && input.peek2(Token![=]) {
            let _ = input.parse::<Token![@]>()?;
            let _ = input.parse::<Token![=]>()?;
            true
        } else {
            let _ = input.parse::<Token![=]>()?;
            false
        };

        let variants = RuleVariant::parse_list(input, capture_span)?;

        Ok(Rule {
            attrs,
            is_pub,
            name,
            generics,
            params,
            return_type,
            variants,
        })
    }
}

impl ToTokens for Rule {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let attrs = &self.attrs;
        let vis = &self.is_pub;
        let name = &self.name;
        let generics = &self.generics;
        let ret = &self.return_type;
        let variants = &self.variants;

        let params_tokens = if self.params.is_empty() {
            quote! {}
        } else {
            let params = &self.params;
            quote! { ( #(#params),* ) }
        };

        let mut variants_tokens = TokenStream::new();
        for (i, v) in variants.iter().enumerate() {
            if i > 0 {
                token::Or::default().to_tokens(&mut variants_tokens);
            }
            v.to_tokens(&mut variants_tokens);
        }

        // We don't have explicit access to capture_span here to re-emit it,
        // but RuleVariant knows about with_span which is derived from it.
        // However, standard ToTokens for Rule usually reconstructs the syntax.
        // If we want to support round-tripping or accurate ToTokens, we should store capture_span in Rule.
        // But for now, just emitting = is standard. If the variants use it, fine.

        tokens.append_all(quote! {
            #(#attrs)*
            #vis rule #name #generics #params_tokens -> #ret = #variants_tokens
        });
    }
}

impl Rule {
    /// Liest alle aufeinanderfolgenden Muster einer Sequenz.
    pub fn parse_all(input: ParseStream) -> Result<Vec<Self>> {
        let mut rules = Vec::new();
        while !input.is_empty() {
            rules.push(input.parse()?);
        }
        Ok(rules)
    }
}

#[derive(Debug, Clone)]
/// Eine Alternative einer Regel - syntaktische Form.
///
/// Semantisches Gegenstueck: [`crate::model::RuleVariant`].
pub struct RuleVariant {
    /// Die Mustersequenz.
    pub pattern: Vec<Pattern>,
    /// Der Klartextname aus `# "…"`.
    pub label: Option<String>,
    /// Der Aktionsblock hinter `->`.
    pub action: TokenStream,
    /// Soll die Spanne der Alternative gebunden werden?
    pub with_span: bool,
    /// Wurde der Aktionsblock ausgeschrieben (statt ergaenzt)?
    pub is_explicit: bool,
}

impl RuleVariant {
    /// Liest die durch `|` getrennten Alternativen einer Regel.
    ///
    /// `capture_span` schaltet die Spannenbindung fuer alle Alternativen an.
    pub fn parse_list(input: ParseStream, capture_span: bool) -> Result<Vec<Self>> {
        let mut variants = Vec::new();
        loop {
            let mut pattern: Vec<Pattern> = Vec::new();
            while !input.is_empty()
                && !input.peek(Token![->])
                && !input.peek(Token![|])
                && !input.peek(Token![#])
                && !input.peek(kw::rule)
            {
                // Lookahead to detect start of next rule:
                // 1. Ident followed by `=` (e.g. `next_rule = ...`)
                if input.peek(Ident) && input.peek2(Token![=]) {
                    break;
                }
                // 2. Ident followed by `@` then `=`
                if input.peek(Ident) && input.peek2(Token![@]) && input.peek3(Token![=]) {
                    break;
                }
                // 3. `pub` keyword (e.g. `pub rule ...` or `pub next_rule ...`)
                if input.peek(Token![pub]) {
                    break;
                }

                pattern.push(input.parse()?);
            }

            let label = if input.peek(Token![#]) {
                let _ = input.parse::<Token![#]>()?;
                let lit: syn::LitStr = input.parse()?;
                Some(lit.value())
            } else {
                None
            };

            let mut is_explicit = false;
            let action = if input.peek(Token![->]) {
                is_explicit = true;
                let _ = input.parse::<Token![->]>()?;
                let content;
                syn::braced!(content in input);
                content.parse()?
            } else {
                let mut bindings = Vec::new();
                for p in &pattern {
                    p.collect_bindings(&mut bindings);
                }

                if bindings.is_empty() {
                    quote! { () }
                } else if bindings.len() == 1 {
                    let b = &bindings[0];
                    quote! { #b }
                } else {
                    quote! { ( #(#bindings),* ) }
                }
            };

            variants.push(RuleVariant {
                pattern,
                label,
                action,
                with_span: capture_span,
                is_explicit,
            });

            if input.peek(Token![|]) {
                let _ = input.parse::<Token![|]>()?;
            } else {
                break;
            }
        }
        Ok(variants)
    }
}

impl ToTokens for RuleVariant {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let pattern = &self.pattern;
        let action = &self.action;
        let label = if let Some(l) = &self.label {
            let l_lit = syn::LitStr::new(l, proc_macro2::Span::call_site());
            quote! { # #l_lit }
        } else {
            quote! {}
        };

        if self.is_explicit {
            tokens.append_all(quote! {
                #(#pattern)* #label -> { #action }
            });
        } else {
            tokens.append_all(quote! {
                #(#pattern)* #label
            });
        }
    }
}

#[derive(Debug, Clone)]
/// Ein Argument eines Regelaufrufs - syntaktische Form.
///
/// Semantisches Gegenstueck: [`crate::model::Argument`].
pub enum Argument {
    /// Nach Position, etwa `separated(item, ",")`.
    Positional(Pattern),
    /// Nach Name, etwa `min=1`.
    Named(Ident, Pattern),
}

impl Parse for Argument {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(Ident) && input.peek2(Token![=]) {
            let name: Ident = input.parse()?;
            let _ = input.parse::<Token![=]>()?;
            let val: Pattern = input.parse()?;
            Ok(Argument::Named(name, val))
        } else {
            Ok(Argument::Positional(input.parse()?))
        }
    }
}

impl ToTokens for Argument {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Argument::Positional(p) => p.to_tokens(tokens),
            Argument::Named(n, p) => {
                n.to_tokens(tokens);
                token::Eq::default().to_tokens(tokens);
                p.to_tokens(tokens);
            }
        }
    }
}

/// A sequence of patterns with an optional action and label.
pub type GroupAlternative = (Vec<Pattern>, Option<TokenStream>, Option<String>);

#[derive(Debug, Clone)]
/// Ein Muster - syntaktische Form.
///
/// Spiegelt [`crate::model::ModelPattern`], behaelt aber die tatsaechlichen
/// Tokens (`Token![?]`, `kw::peek`, `token::Paren` …). Sie tragen die
/// Quellstelle, aus der im Modell die blossen `Span`s werden. Was die einzelnen
/// Varianten bedeuten, steht dort; hier steht, woraus sie gelesen werden.
pub enum Pattern {
    /// `=>` - Cut.
    Cut(Token![=>]),
    /// Ein Literalterminal, etwa `"fn"`.
    Lit {
        /// Bindungsname aus `name:"fn"`.
        binding: Option<Ident>,
        /// Das Literal.
        lit: Lit,
    },
    /// Aufruf einer Regel, eines Builtins oder eines `syn::`-Typs.
    RuleCall {
        /// Bindungsname.
        binding: Option<Ident>,
        /// Der aufgerufene Pfad.
        rule_path: Path,
        /// Generische Argumente in `<…>`.
        generics: Vec<Type>,
        /// Argumente in `( … )`.
        args: Vec<Argument>,
    },
    /// `( a | b )` - geklammerte Alternativenmenge.
    Group {
        /// Bindungsname der Gruppe.
        binding: Option<Ident>,
        /// Die Alternativen.
        alts: Vec<GroupAlternative>,
        /// Die Klammern selbst.
        token: token::Paren,
    },
    /// `bracket( … )` - Inhalt echter eckiger Klammern.
    Bracketed {
        /// Bindungsname.
        binding: Option<Ident>,
        /// Die Muster im Inneren.
        patterns: Vec<Pattern>,
        /// Die eckigen Klammern.
        token: token::Bracket,
    },
    /// `{ … }` - Inhalt echter geschweifter Klammern.
    Braced {
        /// Bindungsname.
        binding: Option<Ident>,
        /// Die Muster im Inneren.
        patterns: Vec<Pattern>,
        /// Die geschweiften Klammern.
        token: token::Brace,
    },
    /// `paren( … )` - Inhalt echter runder Klammern.
    Parenthesized {
        /// Bindungsname.
        binding: Option<Ident>,
        /// Die Muster im Inneren.
        patterns: Vec<Pattern>,
        /// Das Schluesselwort `paren`.
        kw_token: kw::paren,
        /// Die runden Klammern dahinter.
        token: token::Paren,
    },
    /// `x?`
    Optional(Box<Pattern>, Token![?]),
    /// `x*`
    Repeat(Box<Pattern>, Token![*]),
    /// `x+`
    Plus(Box<Pattern>, Token![+]),
    /// `x @ name` - bindet die Spanne an `name`.
    SpanBinding(Box<Pattern>, Ident, Token![@]),
    /// `recover(body, sync)`
    Recover {
        /// Bindungsname; das Ergebnis ist ein `Option`.
        binding: Option<Ident>,
        /// Das eigentliche Muster.
        body: Box<Pattern>,
        /// Die Synchronisationsmarke.
        sync: Box<Pattern>,
        /// Das Schluesselwort `recover`.
        kw_token: kw::recover,
    },
    /// `peek(x)` - prueft, ohne zu verbrauchen.
    Peek(Box<Pattern>, kw::peek),
    /// `not(x)` - schlaegt fehl, wenn `x` passen wuerde.
    Not(Box<Pattern>, kw::not),
    /// `until(x)` - sammelt rohe Tokens bis `x`.
    Until {
        /// Bindungsname; das Ergebnis ist ein `TokenStream`.
        binding: Option<Ident>,
        /// Das Abbruchmuster.
        pattern: Box<Pattern>,
        /// Das Schluesselwort `until`.
        kw_token: kw::until,
    },
    /// `count(x)` - zaehlt Treffer des Elements.
    Count {
        /// Bindungsname; das Ergebnis ist ein `usize`.
        binding: Option<Ident>,
        /// Das gezaehlte Muster.
        pattern: Box<Pattern>,
        /// Das Schluesselwort `count`.
        kw_token: kw::count,
    },
    /// `lex( … )` - zeichengenauer Bereich.
    LexicalScope(Box<Pattern>, kw::lex),
    /// `spaced( … )` - Bereich, in dem Leerraum zaehlt.
    SpacedScope(Box<Pattern>, kw::spaced),
    /// `fail("…")` - bricht mit dieser Meldung ab.
    Fail {
        /// Die Meldung.
        message: Option<Lit>,
        /// Das Schluesselwort `fail`.
        kw_token: kw::fail,
    },
}

impl Pattern {
    fn wrap_sequence(patterns: Vec<Pattern>) -> Pattern {
        if patterns.len() == 1 {
            patterns.into_iter().next().unwrap()
        } else {
            Pattern::Group {
                binding: None,
                alts: vec![(patterns, None, None)],
                token: token::Paren::default(),
            }
        }
    }

    /// Haengt alle Bindungsnamen dieses Musters an `acc` an, in Reihenfolge.
    pub fn collect_bindings(&self, acc: &mut Vec<Ident>) {
        match self {
            Pattern::Lit { binding, .. } => {
                if let Some(b) = binding {
                    acc.push(b.clone());
                }
            }
            Pattern::RuleCall { binding, .. } => {
                if let Some(b) = binding {
                    acc.push(b.clone());
                }
            }
            Pattern::Group { binding, alts, .. } => {
                if let Some(b) = binding {
                    acc.push(b.clone());
                } else {
                    for (pats, action, _) in alts {
                        if action.is_none() {
                            for p in pats {
                                p.collect_bindings(acc);
                            }
                        }
                    }
                }
            }
            Pattern::Bracketed {
                binding, patterns, ..
            } => {
                if let Some(b) = binding {
                    acc.push(b.clone());
                } else {
                    for p in patterns {
                        p.collect_bindings(acc);
                    }
                }
            }
            Pattern::Braced {
                binding, patterns, ..
            } => {
                if let Some(b) = binding {
                    acc.push(b.clone());
                } else {
                    for p in patterns {
                        p.collect_bindings(acc);
                    }
                }
            }
            Pattern::Parenthesized {
                binding, patterns, ..
            } => {
                if let Some(b) = binding {
                    acc.push(b.clone());
                } else {
                    for p in patterns {
                        p.collect_bindings(acc);
                    }
                }
            }
            Pattern::Optional(p, _) => p.collect_bindings(acc),
            Pattern::Repeat(p, _) => p.collect_bindings(acc),
            Pattern::Plus(p, _) => p.collect_bindings(acc),
            Pattern::SpanBinding(p, id, _) => {
                acc.push(id.clone());
                p.collect_bindings(acc);
            }
            Pattern::Recover { binding, body, .. } => {
                if let Some(b) = binding {
                    acc.push(b.clone());
                } else {
                    body.collect_bindings(acc);
                }
            }
            Pattern::Peek(p, _) => p.collect_bindings(acc),
            Pattern::Not(p, _) => p.collect_bindings(acc),
            Pattern::Until {
                binding, pattern, ..
            } => {
                if let Some(b) = binding {
                    acc.push(b.clone());
                } else {
                    pattern.collect_bindings(acc);
                }
            }
            Pattern::Count {
                binding, pattern, ..
            } => {
                if let Some(b) = binding {
                    acc.push(b.clone());
                } else {
                    pattern.collect_bindings(acc);
                }
            }
            Pattern::LexicalScope(p, _) => p.collect_bindings(acc),
            Pattern::SpacedScope(p, _) => p.collect_bindings(acc),
            Pattern::Fail { .. } => {}
            Pattern::Cut(_) => {}
        }
    }

    /// Bindet dieses Muster - oder eines darin - einen Namen?
    pub fn has_binding(&self) -> bool {
        match self {
            Pattern::Lit { binding, .. } => binding.is_some(),
            Pattern::RuleCall { binding, .. } => binding.is_some(),
            Pattern::Group { binding, alts, .. } => {
                if binding.is_some() {
                    return true;
                }
                alts.iter().any(|(pats, action, _)| {
                    action.is_none() && pats.iter().any(|p| p.has_binding())
                })
            }
            Pattern::Bracketed {
                binding, patterns, ..
            } => binding.is_some() || patterns.iter().any(|p| p.has_binding()),
            Pattern::Braced {
                binding, patterns, ..
            } => binding.is_some() || patterns.iter().any(|p| p.has_binding()),
            Pattern::Parenthesized {
                binding, patterns, ..
            } => binding.is_some() || patterns.iter().any(|p| p.has_binding()),
            Pattern::Optional(p, _) => p.has_binding(),
            Pattern::Repeat(p, _) => p.has_binding(),
            Pattern::Plus(p, _) => p.has_binding(),
            Pattern::SpanBinding(..) => true,
            Pattern::Recover {
                binding,
                body,
                sync,
                ..
            } => binding.is_some() || body.has_binding() || sync.has_binding(),
            Pattern::Peek(p, _) => p.has_binding(),
            Pattern::Not(p, _) => p.has_binding(),
            Pattern::Until {
                binding, pattern, ..
            } => binding.is_some() || pattern.has_binding(),
            Pattern::Count {
                binding, pattern, ..
            } => binding.is_some() || pattern.has_binding(),
            Pattern::LexicalScope(p, _) => p.has_binding(),
            Pattern::SpacedScope(p, _) => p.has_binding(),
            Pattern::Fail { .. } => false,
            Pattern::Cut(_) => false,
        }
    }
}

impl Parse for Pattern {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut pat = parse_atom(input)?;

        loop {
            if input.peek(Token![*]) {
                let token = input.parse::<Token![*]>()?;
                pat = Pattern::Repeat(Box::new(pat), token);
            } else if input.peek(Token![+]) {
                let token = input.parse::<Token![+]>()?;
                pat = Pattern::Plus(Box::new(pat), token);
            } else if input.peek(Token![?]) {
                let token = input.parse::<Token![?]>()?;
                pat = Pattern::Optional(Box::new(pat), token);
            } else if input.peek(Token![@]) {
                let token = input.parse::<Token![@]>()?;
                let ident = input.parse::<Ident>()?;
                pat = Pattern::SpanBinding(Box::new(pat), ident, token);
            } else {
                break;
            }
        }
        Ok(pat)
    }
}

impl ToTokens for Pattern {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Pattern::Cut(_) => {
                token::FatArrow::default().to_tokens(tokens);
            }
            Pattern::Lit { binding, lit } => {
                if let Some(b) = binding {
                    b.to_tokens(tokens);
                    token::Colon::default().to_tokens(tokens);
                }
                lit.to_tokens(tokens);
            }
            Pattern::RuleCall {
                binding,
                rule_path,
                generics,
                args,
            } => {
                if let Some(b) = binding {
                    b.to_tokens(tokens);
                    token::Colon::default().to_tokens(tokens);
                }
                rule_path.to_tokens(tokens);
                if !generics.is_empty() {
                    token::Lt::default().to_tokens(tokens);
                    for (i, t) in generics.iter().enumerate() {
                        if i > 0 {
                            token::Comma::default().to_tokens(tokens);
                        }
                        t.to_tokens(tokens);
                    }
                    token::Gt::default().to_tokens(tokens);
                }
                if !args.is_empty() {
                    token::Paren::default().surround(tokens, |t| {
                        for (i, a) in args.iter().enumerate() {
                            if i > 0 {
                                token::Comma::default().to_tokens(t);
                            }
                            a.to_tokens(t);
                        }
                    });
                }
            }
            Pattern::Group { binding, alts, .. } => {
                if let Some(b) = binding {
                    b.to_tokens(tokens);
                    token::Colon::default().to_tokens(tokens);
                }
                token::Paren::default().surround(tokens, |t| {
                    for (i, (seq, action, label)) in alts.iter().enumerate() {
                        if i > 0 {
                            token::Or::default().to_tokens(t);
                        }
                        for p in seq {
                            p.to_tokens(t);
                        }
                        if let Some(a) = action {
                            token::RArrow::default().to_tokens(t);
                            token::Brace::default().surround(t, |t2| a.to_tokens(t2));
                        }
                        if let Some(l) = label {
                            token::Pound::default().to_tokens(t);
                            syn::LitStr::new(l, proc_macro2::Span::call_site()).to_tokens(t);
                        }
                    }
                });
            }
            Pattern::Bracketed {
                binding, patterns, ..
            } => {
                if let Some(b) = binding {
                    b.to_tokens(tokens);
                    token::Colon::default().to_tokens(tokens);
                }
                token::Bracket::default().surround(tokens, |t| {
                    for p in patterns {
                        p.to_tokens(t);
                    }
                });
            }
            Pattern::Braced {
                binding, patterns, ..
            } => {
                if let Some(b) = binding {
                    b.to_tokens(tokens);
                    token::Colon::default().to_tokens(tokens);
                }
                token::Brace::default().surround(tokens, |t| {
                    for p in patterns {
                        p.to_tokens(t);
                    }
                });
            }
            Pattern::Parenthesized {
                binding, patterns, ..
            } => {
                if let Some(b) = binding {
                    b.to_tokens(tokens);
                    token::Colon::default().to_tokens(tokens);
                }
                kw::paren::default().to_tokens(tokens);
                token::Paren::default().surround(tokens, |t| {
                    for p in patterns {
                        p.to_tokens(t);
                    }
                });
            }
            Pattern::Optional(p, _) => {
                p.to_tokens(tokens);
                token::Question::default().to_tokens(tokens);
            }
            Pattern::Repeat(p, _) => {
                p.to_tokens(tokens);
                token::Star::default().to_tokens(tokens);
            }
            Pattern::Plus(p, _) => {
                p.to_tokens(tokens);
                token::Plus::default().to_tokens(tokens);
            }
            Pattern::SpanBinding(p, id, _) => {
                p.to_tokens(tokens);
                token::At::default().to_tokens(tokens);
                id.to_tokens(tokens);
            }
            Pattern::Recover {
                binding,
                body,
                sync,
                ..
            } => {
                if let Some(b) = binding {
                    b.to_tokens(tokens);
                    token::Colon::default().to_tokens(tokens);
                }
                kw::recover::default().to_tokens(tokens);
                token::Paren::default().surround(tokens, |t| {
                    body.to_tokens(t);
                    token::Comma::default().to_tokens(t);
                    sync.to_tokens(t);
                });
            }
            Pattern::Peek(p, _) => {
                kw::peek::default().to_tokens(tokens);
                token::Paren::default().surround(tokens, |t| {
                    p.to_tokens(t);
                });
            }
            Pattern::Not(p, _) => {
                kw::not::default().to_tokens(tokens);
                token::Paren::default().surround(tokens, |t| {
                    p.to_tokens(t);
                });
            }
            Pattern::Until {
                binding, pattern, ..
            } => {
                if let Some(b) = binding {
                    b.to_tokens(tokens);
                    token::Colon::default().to_tokens(tokens);
                }
                kw::until::default().to_tokens(tokens);
                token::Paren::default().surround(tokens, |t| {
                    pattern.to_tokens(t);
                });
            }
            Pattern::Count {
                binding, pattern, ..
            } => {
                if let Some(b) = binding {
                    b.to_tokens(tokens);
                    token::Colon::default().to_tokens(tokens);
                }
                kw::count::default().to_tokens(tokens);
                token::Paren::default().surround(tokens, |t| {
                    pattern.to_tokens(t);
                });
            }
            Pattern::LexicalScope(pattern, kw_token) => {
                kw_token.to_tokens(tokens);
                token::Paren::default().surround(tokens, |t| {
                    pattern.to_tokens(t);
                });
            }
            Pattern::SpacedScope(pattern, kw_token) => {
                kw_token.to_tokens(tokens);
                token::Paren::default().surround(tokens, |t| {
                    pattern.to_tokens(t);
                });
            }
            Pattern::Fail { message, kw_token } => {
                kw_token.to_tokens(tokens);
                if let Some(m) = message {
                    token::Paren::default().surround(tokens, |t| {
                        m.to_tokens(t);
                    });
                }
            }
        }
    }
}

fn parse_atom(input: ParseStream) -> Result<Pattern> {
    // 1. Check for binding
    let binding = rt::attempt(input, |input| {
        let id: Ident = input.parse()?;
        let _ = input.parse::<Token![:]>()?;
        Ok(id)
    })?;

    if input.peek(Token![=>]) {
        if binding.is_some() {
            return Err(input.error("Cut operator cannot be bound."));
        }
        let token = input.parse::<Token![=>]>()?;
        Ok(Pattern::Cut(token))
    } else if input.peek(Token![!]) {
        Err(input
            .error("The '!' operator is not supported. Use 'not(pattern)' for negative lookahead."))
    } else if input.peek(Token![&]) {
        Err(input.error(
            "The '&' operator is not supported. Use 'peek(pattern)' for positive lookahead.",
        ))
    } else if input.peek(Token![~]) {
        Err(input.error("The '~' operator is not supported. Use the '=>' cut operator instead"))
    } else if input.peek(Lit) {
        let lit: Lit = input.parse()?;
        // Char literals are preserved as is.
        Ok(Pattern::Lit { binding, lit })
    } else if input.peek(token::Bracket) {
        let content;
        let token = syn::bracketed!(content in input);
        Ok(Pattern::Bracketed {
            binding,
            patterns: parse_pattern_list(&content)?,
            token,
        })
    } else if input.peek(token::Brace) {
        let content;
        let token = syn::braced!(content in input);
        Ok(Pattern::Braced {
            binding,
            patterns: parse_pattern_list(&content)?,
            token,
        })
    } else if input.peek(kw::paren) {
        let kw = input.parse::<kw::paren>()?;
        let content;
        let token = syn::parenthesized!(content in input);
        Ok(Pattern::Parenthesized {
            binding,
            patterns: parse_pattern_list(&content)?,
            kw_token: kw,
            token,
        })
    } else if input.peek(token::Paren) {
        let content;
        let token = syn::parenthesized!(content in input);
        Ok(Pattern::Group {
            binding,
            alts: parse_group_content(&content)?,
            token,
        })
    } else if input.peek(kw::recover) {
        let kw_token = input.parse::<kw::recover>()?;
        let content;
        syn::parenthesized!(content in input);
        let body = content.parse()?;
        let _ = content.parse::<Token![,]>()?;
        let sync = content.parse()?;
        Ok(Pattern::Recover {
            binding,
            body: Box::new(body),
            sync: Box::new(sync),
            kw_token,
        })
    } else if input.peek(kw::peek) {
        if binding.is_some() {
            return Err(input.error("Peek cannot be bound."));
        }
        let kw_token = input.parse::<kw::peek>()?;
        let content;
        syn::parenthesized!(content in input);
        let inner = content.parse()?;
        Ok(Pattern::Peek(Box::new(inner), kw_token))
    } else if input.peek(kw::not) {
        if binding.is_some() {
            return Err(input.error("Not cannot be bound."));
        }
        let kw_token = input.parse::<kw::not>()?;
        let content;
        syn::parenthesized!(content in input);
        let inner = content.parse()?;
        Ok(Pattern::Not(Box::new(inner), kw_token))
    } else if input.peek(kw::until) {
        let kw_token = input.parse::<kw::until>()?;
        let content;
        syn::parenthesized!(content in input);
        let pattern = content.parse()?;
        Ok(Pattern::Until {
            binding,
            pattern: Box::new(pattern),
            kw_token,
        })
    } else if input.peek(kw::count) {
        let kw_token = input.parse::<kw::count>()?;
        let content;
        syn::parenthesized!(content in input);
        let inner = content.parse()?;
        Ok(Pattern::Count {
            binding,
            pattern: Box::new(inner),
            kw_token,
        })
    } else if input.peek(kw::lex) {
        let kw_token = input.parse::<kw::lex>()?;
        let content;
        syn::parenthesized!(content in input);
        let patterns = parse_pattern_list(&content)?;
        Ok(Pattern::LexicalScope(
            Box::new(Pattern::wrap_sequence(patterns)),
            kw_token,
        ))
    } else if input.peek(kw::spaced) {
        let kw_token = input.parse::<kw::spaced>()?;
        let content;
        syn::parenthesized!(content in input);
        let patterns = parse_pattern_list(&content)?;
        Ok(Pattern::SpacedScope(
            Box::new(Pattern::wrap_sequence(patterns)),
            kw_token,
        ))
    } else if input.peek(kw::fail) {
        if binding.is_some() {
            return Err(input.error("Fail cannot be bound."));
        }
        let kw_token = input.parse::<kw::fail>()?;
        let message = if input.peek(token::Paren) {
            let content;
            syn::parenthesized!(content in input);
            if content.is_empty() {
                None
            } else {
                Some(content.parse()?)
            }
        } else {
            None
        };
        Ok(Pattern::Fail { message, kw_token })
    } else {
        let rule_path = parse_path_no_args(input)?;

        let generics = if input.peek(Token![<]) {
            let _ = input.parse::<Token![<]>()?;
            let mut types = Vec::new();
            if !input.peek(Token![>]) {
                loop {
                    types.push(input.parse::<Type>()?);
                    if input.peek(Token![,]) {
                        let _ = input.parse::<Token![,]>()?;
                        if input.peek(Token![>]) {
                            break;
                        }
                    } else {
                        break;
                    }
                }
            }
            let _gt_token = input.parse::<Token![>]>()?;
            types
        } else {
            Vec::new()
        };

        let args = if !generics.is_empty() {
            if input.peek(token::Paren) {
                parse_args(input)?
            } else {
                Vec::new()
            }
        } else if input.peek(token::Paren) {
            // Simplified Disambiguation logic:
            // 1. `name = value` -> Always allowed (Arguments).
            // 2. Built-in rules -> Always allowed (Arguments).
            // 3. Positional args for user rules -> DISALLOWED (defaults to empty args -> Group).

            let fork = input.fork();
            let content;
            syn::parenthesized!(content in fork);
            let has_named_arg = content.peek(Ident) && content.peek2(Token![=]);

            let is_simple_ident =
                rule_path.segments.len() == 1 && rule_path.leading_colon.is_none();
            let ident_str = if is_simple_ident {
                rule_path.segments[0].ident.to_string()
            } else {
                String::new()
            };
            let is_builtin =
                is_simple_ident && (ident_str == "separated" || ident_str == "repeated");

            // Note: `is_scoped` (e.g. `foo::bar(...)`) is NO LONGER a heuristic for args.
            // Explicitly: only built-ins or named args or templates allowed.

            if has_named_arg || is_builtin {
                parse_args(input)?
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        Ok(Pattern::RuleCall {
            binding,
            rule_path,
            generics,
            args,
        })
    }
}

fn parse_args(input: ParseStream) -> Result<Vec<Argument>> {
    let mut args = Vec::new();
    if input.peek(token::Paren) {
        let content;
        syn::parenthesized!(content in input);
        while !content.is_empty() {
            args.push(content.parse()?);
            if content.peek(Token![,]) {
                let _ = content.parse::<Token![,]>()?;
            }
        }
    }
    Ok(args)
}

fn parse_pattern_list(input: ParseStream) -> Result<Vec<Pattern>> {
    let mut list = Vec::new();
    while !input.is_empty() {
        list.push(input.parse()?);
    }
    Ok(list)
}

fn parse_group_content(input: ParseStream) -> Result<Vec<GroupAlternative>> {
    let mut alts = Vec::new();
    loop {
        let mut seq = Vec::new();
        while !input.is_empty()
            && !input.peek(Token![|])
            && !input.peek(Token![#])
            && !input.peek(Token![->])
        {
            seq.push(input.parse()?);
        }

        let action = if input.peek(Token![->]) {
            let _: Token![->] = input.parse()?;
            let content;
            syn::braced!(content in input);
            Some(content.parse()?)
        } else {
            None
        };

        let label = if input.peek(Token![#]) {
            let _: Token![#] = input.parse()?;
            let lit: syn::LitStr = input.parse()?;
            Some(lit.value())
        } else {
            None
        };

        alts.push((seq, action, label));
        if input.peek(Token![|]) {
            let _: Token![|] = input.parse()?;
        } else {
            break;
        }
    }
    Ok(alts)
}
