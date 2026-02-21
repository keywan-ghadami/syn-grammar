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

pub mod kw {
    syn::custom_keyword!(grammar);
    syn::custom_keyword!(ruleset);
    syn::custom_keyword!(rule);
    syn::custom_keyword!(paren);
    syn::custom_keyword!(recover);
    syn::custom_keyword!(peek);
    syn::custom_keyword!(not);
    syn::custom_keyword!(until);
}

#[derive(Debug, Clone)]
pub struct RuleSet {
    pub alias: Ident,
    pub rules: Vec<Rule>,
}

impl Parse for RuleSet {
    fn parse(input: ParseStream) -> Result<Self> {
        let _ = input.parse::<kw::ruleset>()?;
        let content;
        let _ = syn::braced!(content in input);
        let mut rules = Rule::parse_all(&content)?;
        let _ = input.parse::<Token![as]>()?;
        let alias: Ident = input.parse()?;
        let _ = input.parse::<Token![;]>()?;

        // Mangle rules
        for rule in &mut rules {
            mangle_rule(rule, &alias);
        }

        Ok(RuleSet { alias, rules })
    }
}

#[derive(Debug, Clone)]
pub struct GrammarDefinition {
    pub name: Ident,
    pub inherits: Option<InheritanceSpec>,
    pub uses: Vec<ItemUse>,
    pub rules: Vec<Rule>,
}

impl Parse for GrammarDefinition {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut rules = Vec::new();

        // Support rulesets before grammar
        while input.peek(kw::ruleset) {
            let set: RuleSet = input.parse()?;
            rules.extend(set.rules);
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
        while content.peek(Token![use]) {
            uses.push(content.parse()?);
        }

        // Parse rulesets inside the grammar block
        while content.peek(kw::ruleset) {
            let set: RuleSet = content.parse()?;
            rules.extend(set.rules);
        }

        let main_rules = Rule::parse_all(&content)?;
        rules.extend(main_rules);

        // Support rulesets after grammar
        while input.peek(kw::ruleset) {
            let set: RuleSet = input.parse()?;
            rules.extend(set.rules);
        }

        Ok(GrammarDefinition {
            name,
            inherits,
            uses,
            rules,
        })
    }
}

impl ToTokens for GrammarDefinition {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let name = &self.name;
        let inherits = &self.inherits;
        let uses = &self.uses;
        let rules = &self.rules;

        tokens.append_all(quote! {
            grammar #name #inherits {
                #(#uses)*
                #(#rules)*
            }
        });
    }
}

fn mangle_rule(rule: &mut Rule, alias: &Ident) {
    // Mangle rule definition name: name -> alias__name
    let old_name = rule.name.clone();
    let new_name = syn::Ident::new(&format!("{}__{}", alias, old_name), old_name.span());
    rule.name = new_name;

    // Mangle recursive calls inside patterns
    for variant in &mut rule.variants {
        for pattern in &mut variant.pattern {
            mangle_pattern(pattern, alias);
        }
    }
}

fn mangle_pattern(pattern: &mut Pattern, alias: &Ident) {
    match pattern {
        Pattern::RuleCall {
            rule_path, args, ..
        } => {
            // Only mangle if it's a simple identifier (local call)
            if rule_path.segments.len() == 1 {
                let segment = &mut rule_path.segments[0];
                if segment.arguments.is_none() {
                    let old_ident = segment.ident.clone();
                    // We change it to alias::old_ident
                    let new_path: Path = syn::parse_quote!(#alias::#old_ident);
                    *rule_path = new_path;
                }
            }

            for arg in args {
                match arg {
                    Argument::Positional(p) | Argument::Named(_, p) => mangle_pattern(p, alias),
                }
            }
        }
        Pattern::Group(alts, _) => {
            for (seq, _) in alts {
                for p in seq {
                    mangle_pattern(p, alias);
                }
            }
        }
        Pattern::Bracketed(seq, _)
        | Pattern::Braced(seq, _)
        | Pattern::Parenthesized(seq, _, _) => {
            for p in seq {
                mangle_pattern(p, alias);
            }
        }
        Pattern::Optional(p, _)
        | Pattern::Repeat(p, _)
        | Pattern::Plus(p, _)
        | Pattern::SpanBinding(p, _, _)
        | Pattern::Peek(p, _)
        | Pattern::Not(p, _)
        | Pattern::Until { pattern: p, .. } => {
            mangle_pattern(p, alias);
        }
        Pattern::Recover { body, sync, .. } => {
            mangle_pattern(body, alias);
            mangle_pattern(sync, alias);
        }
        _ => {}
    }
}

#[derive(Debug, Clone)]
pub struct InheritanceSpec {
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
pub struct RuleParameter {
    pub name: Ident,
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
pub struct Rule {
    pub attrs: Vec<Attribute>,
    pub is_pub: Option<Token![pub]>,
    pub name: Ident,
    pub generics: Generics,
    pub params: Vec<RuleParameter>,
    pub return_type: Type,
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

        let _ = input.parse::<kw::rule>()?;
        let name = rt::parse_ident(input)?;

        // Parse generics if present (e.g., <T, U>)
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
        let _ = input.parse::<Token![=]>()?;

        let variants = RuleVariant::parse_list(input)?;

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

        // Join variants with |
        let mut variants_tokens = TokenStream::new();
        for (i, v) in variants.iter().enumerate() {
            if i > 0 {
                token::Or::default().to_tokens(&mut variants_tokens);
            }
            v.to_tokens(&mut variants_tokens);
        }

        tokens.append_all(quote! {
            #(#attrs)*
            #vis rule #name #generics #params_tokens -> #ret = #variants_tokens
        });
    }
}

impl Rule {
    pub fn parse_all(input: ParseStream) -> Result<Vec<Self>> {
        let mut rules = Vec::new();
        while !input.is_empty() {
            rules.push(input.parse()?);
        }
        Ok(rules)
    }
}

#[derive(Debug, Clone)]
pub struct RuleVariant {
    pub pattern: Vec<Pattern>,
    pub label: Option<String>,
    pub action: TokenStream,
}

impl RuleVariant {
    pub fn parse_list(input: ParseStream) -> Result<Vec<Self>> {
        let mut variants = Vec::new();
        loop {
            let mut pattern = Vec::new();
            while !input.peek(Token![->]) && !input.peek(Token![|]) && !input.peek(Token![#]) {
                pattern.push(input.parse()?);
            }

            let label = if input.peek(Token![#]) {
                let _ = input.parse::<Token![#]>()?;
                let lit: syn::LitStr = input.parse()?;
                Some(lit.value())
            } else {
                None
            };

            let _ = input.parse::<Token![->]>()?;

            let content;
            syn::braced!(content in input);
            let action = content.parse()?;

            variants.push(RuleVariant {
                pattern,
                label,
                action,
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

        tokens.append_all(quote! {
            #(#pattern)* #label -> { #action }
        });
    }
}

#[derive(Debug, Clone)]
pub enum Argument {
    Positional(Pattern),
    Named(Ident, Pattern),
}

impl Parse for Argument {
    fn parse(input: ParseStream) -> Result<Self> {
        // Check for Named: Ident = ...
        // But Pattern can also start with Ident.
        // Ambiguity: `x` could be a rule call `x` or named arg `x = ...`.
        // We peek for `=` to distinguish.

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

#[derive(Debug, Clone)]
pub enum Pattern {
    Cut(Token![=>]),
    Lit {
        binding: Option<Ident>,
        lit: Lit,
    },
    RuleCall {
        binding: Option<Ident>,
        rule_path: Path,
        generics: Vec<Type>,
        args: Vec<Argument>,
    },
    Group(Vec<(Vec<Pattern>, Option<String>)>, token::Paren),
    Bracketed(Vec<Pattern>, token::Bracket),
    Braced(Vec<Pattern>, token::Brace),
    Parenthesized(Vec<Pattern>, kw::paren, token::Paren),
    Optional(Box<Pattern>, Token![?]),
    Repeat(Box<Pattern>, Token![*]),
    Plus(Box<Pattern>, Token![+]),
    SpanBinding(Box<Pattern>, Ident, Token![@]),
    Recover {
        binding: Option<Ident>,
        body: Box<Pattern>,
        sync: Box<Pattern>,
        kw_token: kw::recover,
    },
    Peek(Box<Pattern>, kw::peek),
    Not(Box<Pattern>, kw::not),
    Until {
        binding: Option<Ident>,
        pattern: Box<Pattern>,
        kw_token: kw::until,
    },
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
                    token::Not::default().to_tokens(tokens);
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
            Pattern::Group(alts, _) => {
                token::Paren::default().surround(tokens, |t| {
                    for (i, (seq, label)) in alts.iter().enumerate() {
                        if i > 0 {
                            token::Or::default().to_tokens(t);
                        }
                        for p in seq {
                            p.to_tokens(t);
                        }
                        if let Some(l) = label {
                            token::Pound::default().to_tokens(t);
                            syn::LitStr::new(l, proc_macro2::Span::call_site()).to_tokens(t);
                        }
                    }
                });
            }
            Pattern::Bracketed(seq, _) => {
                token::Bracket::default().surround(tokens, |t| {
                    for p in seq {
                        p.to_tokens(t);
                    }
                });
            }
            Pattern::Braced(seq, _) => {
                token::Brace::default().surround(tokens, |t| {
                    for p in seq {
                        p.to_tokens(t);
                    }
                });
            }
            Pattern::Parenthesized(seq, _, _) => {
                kw::paren::default().to_tokens(tokens);
                token::Paren::default().surround(tokens, |t| {
                    for p in seq {
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
    } else if input.peek(Lit) {
        let lit: Lit = input.parse()?;
        // Convert char literals to string literals for consistency
        let lit = match lit {
            Lit::Char(c) => Lit::Str(syn::LitStr::new(&c.value().to_string(), c.span())),
            _ => lit,
        };
        Ok(Pattern::Lit { binding, lit })
    } else if input.peek(token::Bracket) {
        if binding.is_some() {
            return Err(input.error("Bracketed groups cannot be bound directly."));
        }
        let content;
        let token = syn::bracketed!(content in input);
        Ok(Pattern::Bracketed(parse_pattern_list(&content)?, token))
    } else if input.peek(token::Brace) {
        if binding.is_some() {
            return Err(input.error("Braced groups cannot be bound directly."));
        }
        let content;
        let token = syn::braced!(content in input);
        Ok(Pattern::Braced(parse_pattern_list(&content)?, token))
    } else if input.peek(kw::paren) {
        if binding.is_some() {
            return Err(input.error("Parenthesized groups cannot be bound directly."));
        }
        let kw = input.parse::<kw::paren>()?;
        let content;
        let token = syn::parenthesized!(content in input);
        Ok(Pattern::Parenthesized(
            parse_pattern_list(&content)?,
            kw,
            token,
        ))
    } else if input.peek(token::Paren) {
        if binding.is_some() {
            return Err(input.error("Groups cannot be bound directly."));
        }
        let content;
        let token = syn::parenthesized!(content in input);
        Ok(Pattern::Group(parse_group_content(&content)?, token))
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
        // until returns a TokenStream, so it can be bound.
        let kw_token = input.parse::<kw::until>()?;
        let content;
        syn::parenthesized!(content in input);
        let pattern = content.parse()?;
        Ok(Pattern::Until {
            binding,
            pattern: Box::new(pattern),
            kw_token,
        })
    } else {
        let rule_path: Path = input.parse()?;

        let is_simple_ident = rule_path.leading_colon.is_none()
            && rule_path.segments.len() == 1
            && rule_path.segments[0].arguments.is_none();

        if is_simple_ident {
            let rule_name = &rule_path.segments[0].ident;
            // Check for aliases
            let is_alias = get_alias(&rule_name.to_string()).is_some();
            if is_alias {
                // Check if it looks like a rule call (generics or contiguous parens)
                let has_generics = input.peek(Token![<]);
                let has_args = if input.peek(Token![!]) {
                    // Check if it's a macro-like call
                    true
                } else {
                    false
                };

                if !has_generics && !has_args {
                    let token_str = get_alias(&rule_name.to_string()).unwrap();
                    return Ok(Pattern::Lit {
                        binding,
                        lit: Lit::Str(syn::LitStr::new(token_str, rule_name.span())),
                    });
                }
            }
        }

        // Parse generics: rule<T, U>
        let generics = if input.peek(Token![<]) {
            let _ = input.parse::<Token![<]>()?;
            let mut types = Vec::new();
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
            let _gt_token = input.parse::<Token![>]>()?;
            types
        } else {
            Vec::new()
        };

        let args = if input.peek(Token![!]) {
            let _ = input.parse::<Token![!]>()?;
            parse_args(input)?
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

fn parse_group_content(input: ParseStream) -> Result<Vec<(Vec<Pattern>, Option<String>)>> {
    let mut alts = Vec::new();
    loop {
        let mut seq = Vec::new();
        while !input.is_empty() && !input.peek(Token![|]) && !input.peek(Token![#]) {
            seq.push(input.parse()?);
        }

        let label = if input.peek(Token![#]) {
            let _ = input.parse::<Token![#]>()?;
            let lit: syn::LitStr = input.parse()?;
            Some(lit.value())
        } else {
            None
        };

        alts.push((seq, label));
        if input.peek(Token![|]) {
            let _ = input.parse::<Token![|]>()?;
        } else {
            break;
        }
    }
    Ok(alts)
}

fn get_alias(name: &str) -> Option<&'static str> {
    match name {
        "PLUS" => Some("+"),
        "MINUS" => Some("-"),
        "STAR" => Some("*"),
        "SLASH" => Some("/"),
        "DOT" => Some("."),
        "COMMA" => Some(","),
        "SEMI" => Some(";"),
        "COLON" => Some(":"),
        "LPAREN" => Some("("),
        "RPAREN" => Some(")"),
        "LBRACE" => Some("{"),
        "RBRACE" => Some("}"),
        "LBRACKET" => Some("["),
        "RBRACKET" => Some("]"),
        "EQ" => Some("="),
        "LT" => Some("<"),
        "GT" => Some(">"),
        "AND" => Some("&"),
        "OR" => Some("|"),
        "NOT" => Some("!"),
        "POUND" => Some("#"),
        "AT" => Some("@"),
        "DOLLAR" => Some("$"),
        "QUESTION" => Some("?"),
        _ => None,
    }
}
