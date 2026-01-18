use crate::model::*;
use syn::parse::{Parse, ParseStream};
use syn::{braced, bracketed, parenthesized, token, Ident, Token, Result, LitStr};

mod kw {
    syn::custom_keyword!(grammar);
    syn::custom_keyword!(rule);
    syn::custom_keyword!(paren); // Für explizite Token-Klammern
}

impl Parse for GrammarDefinition {
    fn parse(input: ParseStream) -> Result<Self> {
        let _ = input.parse::<kw::grammar>()?;
        let name: Ident = input.parse()?;
        
        let inherits = if input.peek(Token![:]) {
            let _ = input.parse::<Token![:]>()?;
            Some(input.parse()?)
        } else {
            None
        };

        let content;
        let _ = braced!(content in input);
        
        let mut rules = Vec::new();
        while !content.is_empty() {
            rules.push(content.parse()?);
        }

        Ok(GrammarDefinition { name, inherits, rules })
    }
}

impl Parse for Rule {
    fn parse(input: ParseStream) -> Result<Self> {
        let is_pub = input.peek(Token![pub]);
        if is_pub { let _ = input.parse::<Token![pub]>()?; }

        let _ = input.parse::<kw::rule>()?;
        let name: Ident = input.parse()?;
        let _ = input.parse::<Token![->]>()?;
        let return_type: syn::Type = input.parse()?;
        let _ = input.parse::<Token![=]>()?;

        let mut variants = Vec::new();

        loop {
            let mut pattern = Vec::new();
            while !input.peek(Token![->]) && !input.peek(Token![|]) && !input.is_empty() {
                pattern.push(input.parse()?);
            }

            let _ = input.parse::<Token![->]>()?;
            let content;
            let _ = braced!(content in input);
            let action = content.parse()?;

            variants.push(RuleVariant { pattern, action });

            if input.peek(Token![|]) {
                let _ = input.parse::<Token![|]>()?;
            } else {
                break;
            }
        }

        Ok(Rule { is_pub, name, return_type, variants })
    }
}

impl Parse for Pattern {
    fn parse(input: ParseStream) -> Result<Self> {
        // Schritt 1: Atom
        let mut base_pattern = if input.peek(LitStr) {
            Pattern::Lit(input.parse()?)

        } else if input.peek(token::Bracket) {
            // [ ... ] -> syn::bracketed!
            let content;
            let _ = bracketed!(content in input);
            let mut seq = Vec::new();
            while !content.is_empty() {
                seq.push(content.parse()?);
            }
            Pattern::Bracketed(seq)

        } else if input.peek(token::Brace) {
            // { ... } -> syn::braced!
            let content;
            let _ = braced!(content in input);
            let mut seq = Vec::new();
            while !content.is_empty() {
                seq.push(content.parse()?);
            }
            Pattern::Braced(seq)

        } else if input.peek(token::Paren) {
            // ( ... ) -> Logisches Grouping (Standard EBNF Verhalten)
            let content;
            let _ = parenthesized!(content in input);
            
            let mut alternatives = Vec::new();
            loop {
                let mut seq = Vec::new();
                while !content.is_empty() && !content.peek(Token![|]) {
                    seq.push(content.parse()?);
                }
                alternatives.push(seq);

                if content.peek(Token![|]) {
                    let _ = content.parse::<Token![|]>()?;
                } else {
                    break;
                }
            }
            Pattern::Group(alternatives)
            
        } else if input.peek(kw::paren) {
             // paren( ... ) -> Echte Token-Klammern syn::parenthesized!
             let _ = input.parse::<kw::paren>()?;
             let content;
             let _ = parenthesized!(content in input);
             let mut seq = Vec::new();
             while !content.is_empty() {
                 seq.push(content.parse()?);
             }
             Pattern::Parenthesized(seq)

        } else {
            // Rule Call
            let binding = if input.peek2(Token![:]) {
                let b: Ident = input.parse()?;
                let _ = input.parse::<Token![:]>()?;
                Some(b)
            } else {
                None
            };

            let rule_name: Ident = input.parse()?;
            
            let args_content;
            let _ = syn::parenthesized!(args_content in input);
            
            // Argumente (vereinfacht: Literale)
            let mut args = Vec::new();
            while !args_content.is_empty() {
                args.push(args_content.parse()?);
                if args_content.peek(Token![,]) {
                    let _ = args_content.parse::<Token![,]>()?;
                }
            }
            
            Pattern::RuleCall { binding, rule_name, args }
        };

        // Schritt 2: Suffix
        loop {
            if input.peek(Token![?]) {
                let _ = input.parse::<Token![?]>()?;
                base_pattern = Pattern::Optional(Box::new(base_pattern));
            } else if input.peek(Token![*]) {
                let _ = input.parse::<Token![*]>()?;
                base_pattern = Pattern::Repeat(Box::new(base_pattern));
            } else if input.peek(Token![+]) {
                let _ = input.parse::<Token![+]>()?;
                base_pattern = Pattern::Plus(Box::new(base_pattern));
            } else {
                break;
            }
        }

        Ok(base_pattern)
    }
}
