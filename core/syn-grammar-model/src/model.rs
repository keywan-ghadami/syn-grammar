use crate::{analysis, parser};
use proc_macro2::{Ident, TokenStream};
use syn::spanned::Spanned;

/// What a backend brings along in terms of built-in rules.
pub mod backend;
/// Backend-independent value types for action blocks.
pub mod types;

/// A whole grammar, as `grammar! { … }` means it.
#[derive(Debug, Clone)]
pub struct GrammarDefinition {
    /// The name from `grammar Name { … }`; also the name of the generated module.
    pub name: Ident,
    /// The generated rules, in source order.
    pub rules: Vec<Rule>,
    /// Rules from `extern rule …;` - they are **not** generated but written by
    /// hand and only called.
    pub extern_rules: Vec<ExternRule>,
    /// Grammars from `import … as alias;`, whose rules can be called as
    /// `alias::rule`.
    pub imports: Vec<ImportedGrammar>,
    /// `use` statements that are carried over unchanged into the generated module.
    /// A glob among them switches off the "Undefined rule" check - the set of
    /// visible names is then not known at macro time.
    pub uses: Vec<syn::ItemUse>,
}

/// A hand-written rule that the grammar only calls.
///
/// From `extern rule name(p: T) -> R;`. The function must be visible at the
/// grammar's definition site and be named
/// `fn name<'a>(input: &Stream<'a>, p: T) -> StreamResult<'a, R>`.
#[derive(Debug, Clone)]
pub struct ExternRule {
    /// Name of the rule and thus of the function to call.
    pub name: Ident,
    /// Generic parameters of the declaration.
    pub generics: syn::Generics,
    /// Declared parameters; they are passed through after `input`.
    pub params: Vec<RuleParameter>,
    /// The type the function returns.
    pub return_type: syn::Type,
    /// Attributes on the declaration.
    pub attrs: Vec<syn::Attribute>,
}

/// A foreign grammar included via `import … as …;`.
#[derive(Debug, Clone)]
pub struct ImportedGrammar {
    /// Path to the module of the foreign grammar.
    pub path: syn::Path,
    /// The alias name under which its rules are called.
    pub alias: Ident,
}

/// A rule of the grammar.
#[derive(Debug, Clone)]
pub struct Rule {
    /// The rule name. `parse_<name>` and `parse_<name>_impl` are derived from it.
    pub name: Ident,
    /// Generic parameters, e.g. `list<T>`.
    pub generics: syn::Generics,
    /// Runtime parameters, e.g. `value(offset: i32)`.
    pub params: Vec<RuleParameter>,
    /// What the rule calls itself in an error message (`# "a shared struct"`
    /// at the definition). Every call site inherits it unless it carries a
    /// label of its own.
    pub label: Option<String>,
    /// The type after `->`.
    pub return_type: syn::Type,
    /// Precomputed classification of [`Rule::return_type`].
    pub return_type_kind: analysis::ReturnTypeKind,
    /// The alternatives separated by `|`, in source order.
    pub variants: Vec<RuleVariant>,
    /// Is it preceded by `pub`? The rule `main` always counts as public.
    pub is_pub: bool,
    /// Lexical rule: inside it, character precision applies rather than token precision.
    pub is_lexical: bool,
    /// Attributes on the rule; `#[doc]`, `#[cfg]` and the lint attributes
    /// are carried into the generated code.
    pub attrs: Vec<syn::Attribute>,
}

/// A parameter of a rule.
#[derive(Debug, Clone)]
pub struct RuleParameter {
    /// The name under which it is visible in the body.
    pub name: Ident,
    /// The type. `None` means: a **parser** parameter (e.g. `list<T>(item)`),
    /// not a value - it is substituted at macro time, not passed at
    /// runtime.
    pub ty: Option<syn::Type>,
}

/// An alternative of a rule.
#[derive(Debug, Clone)]
pub struct RuleVariant {
    /// The pattern sequence that must match.
    pub pattern: Vec<ModelPattern>,
    /// The action block after `->` that builds the value.
    pub action: TokenStream,
    /// The plain-text name from `# "…"`. If the alternative fails at its
    /// starting position, it replaces the internal message as the expectation.
    pub label: Option<String>,
    /// Should the span of the alternative be supplied as well?
    pub with_span: bool,
    /// Was the action block written out, or did the generator add it?
    /// Only the former may trigger a type check.
    pub is_explicit: bool,
}

/// A pattern of the grammar DSL.
///
/// Each variant corresponds to a notation in `SYNTAX.md`.
#[derive(Debug, Clone)]
pub enum ModelPattern {
    /// `=>` - cut. From here on the derivation is fixed; an error after it
    /// is fatal and no longer allows any other alternative.
    Cut(proc_macro2::Span),
    /// A literal terminal, e.g. `"fn"` or `"::"`.
    Lit {
        /// Binding name from `name:"fn"`.
        binding: Option<Ident>,
        /// The literal itself.
        lit: syn::Lit,
    },
    /// The call of a rule, a builtin or a `syn::` type.
    RuleCall {
        /// Binding name from `name:rule`.
        binding: Option<Ident>,
        /// The path: `rule`, `alias::rule` or `syn::Type`.
        rule_path: syn::Path,
        /// Generic arguments, e.g. `list<Vec<T>>`.
        generics: Vec<syn::Type>,
        /// Arguments in parentheses, e.g. `separated(item, ",")`.
        args: Vec<Argument>,
    },
    /// A parenthesized set of alternatives, `( a | b )`.
    Group {
        /// Binding name of the whole group.
        binding: Option<Ident>,
        /// The alternatives as triples of patterns, action block and label.
        alts: Vec<(Vec<ModelPattern>, Option<TokenStream>, Option<String>)>,
        /// Source location of the group.
        span: proc_macro2::Span,
    },
    /// `bracket( … )` - content of real square brackets.
    Bracketed(Vec<ModelPattern>, proc_macro2::Span),
    /// `brace( … )` or `{ … }` - content of real curly braces.
    Braced(Vec<ModelPattern>, proc_macro2::Span),
    /// `paren( … )` - content of real parentheses.
    Parenthesized(Vec<ModelPattern>, proc_macro2::Span),
    /// `x?` - at most once.
    Optional(Box<ModelPattern>, proc_macro2::Span),
    /// `x*` - any number of times, including zero.
    Repeat(Box<ModelPattern>, proc_macro2::Span),
    /// `x+` - at least once.
    Plus(Box<ModelPattern>, proc_macro2::Span),
    /// `x @ name` - binds the span of the pattern to `name`.
    SpanBinding(Box<ModelPattern>, Ident, proc_macro2::Span),
    /// `recover(body, sync)` - if `body` fails, input is skipped up to `sync`
    /// instead of letting the whole rule fail.
    Recover {
        /// Binding name; the result is an `Option`.
        binding: Option<Ident>,
        /// The actual pattern.
        body: Box<ModelPattern>,
        /// The synchronization mark up to which input is skipped.
        sync: Box<ModelPattern>,
        /// Source location.
        span: proc_macro2::Span,
    },
    /// `peek(x)` - checks without consuming.
    Peek(Box<ModelPattern>, proc_macro2::Span),
    /// `not(x)` - fails if `x` would match. Consumes nothing.
    Not(Box<ModelPattern>, proc_macro2::Span),
    /// `until(x)` - collects raw tokens until `x` would match.
    Until {
        /// Binding name; the result is a `TokenStream`.
        binding: Option<Ident>,
        /// The stop pattern. It is not consumed.
        pattern: Box<ModelPattern>,
        /// Source location.
        span: proc_macro2::Span,
    },
    /// `count(x)` - counts how often the **item** matched.
    Count {
        /// Binding name; the result is a `usize`.
        binding: Option<Ident>,
        /// The counted pattern, including its repetition operator.
        pattern: Box<ModelPattern>,
        /// Source location.
        span: proc_macro2::Span,
    },
    /// `lex( … )` - character-precise treatment within the region.
    LexicalScope(Box<ModelPattern>, proc_macro2::Span),
    /// `spaced( … )` - whitespace between tokens becomes significant.
    SpacedScope(Box<ModelPattern>, proc_macro2::Span),
    /// `fail("…")` - aborts with this message, verbatim and with high priority.
    Fail {
        /// The message. Without it, "Explicit failure" is used.
        message: Option<syn::Lit>,
        /// Source location.
        span: proc_macro2::Span,
    },
}

impl ModelPattern {
    /// The source location of the pattern - for error messages of the generator.
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

/// An argument of a rule call.
#[derive(Debug, Clone)]
pub enum Argument {
    /// By position, e.g. `separated(item, ",")`.
    Positional(ModelPattern),
    /// By name, e.g. `separated(item, ",", min=1)`.
    Named(Ident, ModelPattern),
}

impl From<parser::GrammarDefinition> for GrammarDefinition {
    fn from(p: parser::GrammarDefinition) -> Self {
        GrammarDefinition {
            name: p.name,
            rules: p.rules.into_iter().map(Into::into).collect(),
            extern_rules: p.extern_rules.into_iter().map(Into::into).collect(),
            imports: p.imports.into_iter().map(Into::into).collect(),
            uses: p.uses,
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
            label: p.label,
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
