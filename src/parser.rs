use crate::model::*;
use syn::parse::{Parse, ParseStream};
use syn::{braced, parenthesized, token, Ident, Token, Result, LitStr};

mod kw {
    syn::custom_keyword!(grammar);
    syn::custom_keyword!(rule);
}

impl Parse for GrammarDefinition {
    fn parse(input: ParseStream) -> Result<Self> {
        let _ = input.parse::<kw::grammar>()?;
        let name: Ident = input.parse()?;
        
        // Optional: : Parent
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

        // Hier parsen wir jetzt ECHTE Varianten mit | Separator
        let mut variants = Vec::new();

        loop {
            // 1. Pattern-Sequenz parsen
            let mut pattern = Vec::new();
            // Wir lesen solange Patterns, bis wir auf '->' (Action) oder '|' (nächste Variante) oder Ende stoßen
            while !input.peek(Token![->]) && !input.peek(Token![|]) && !input.is_empty() {
                pattern.push(input.parse()?);
            }

            // 2. Action parsen (-> { ... })
            let _ = input.parse::<Token![->]>()?;
            let content;
            let _ = braced!(content in input);
            let action = content.parse()?; // TokenStream

            variants.push(RuleVariant { pattern, action });

            // Gibt es eine weitere Variante?
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
        // Schritt 1: Das "Atom" parsen (Literal, Gruppe oder RuleCall)
        let mut base_pattern = if input.peek(LitStr) {
            // Fall A: String Literal "fn"
            Pattern::Lit(input.parse()?)

        } else if input.peek(token::Paren) {
            // Fall B: Gruppierung ( "a" | "b" )
            let content;
            let _ = parenthesized!(content in input);
            
            // Innerhalb der Klammern parsen wir Alternativen ( | getrennt)
            // Jede Alternative ist eine Sequenz von Patterns
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

        } else {
            // Fall C: Rule Call name:ident()
            // Check binding: v:int_lit()
            let binding = if input.peek2(Token![:]) {
                let b: Ident = input.parse()?;
                let _ = input.parse::<Token![:]>()?;
                Some(b)
            } else {
                None
            };

            let rule_name: Ident = input.parse()?;
            
            // Args (...)
            let args_content;
            let _ = syn::parenthesized!(args_content in input);
            // Argumente parsen (für Stage 0 leer/ignoriert oder simple Literale)
            // Hier könnten wir später args_content.parse_terminated(...) einbauen
            
            Pattern::RuleCall { binding, rule_name, args: vec![] }
        };

        // Schritt 2: Suffix-Operatoren parsen (?, *, +)
        // Wir loopen hier, um theoretisch Dinge wie rule()*? zu erlauben (obwohl das semantisch oft Quatsch ist)
        // oder um Pattern-Precedence korrekt abzubilden (Postfix bindet stark).
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

