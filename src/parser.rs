use crate::model::*;
use syn::parse::{Parse, ParseStream};
use syn::{braced, token, Ident, Token, Result, LitStr};

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

        // Wir parsen hier vereinfacht EINE Variante (kein | Support im Bootstrap-Parser selbst)
        // Das ist der "NQP"-Ansatz: Der Bootstrap-Parser ist dumm.
        let mut pattern = Vec::new();
        while !input.peek(Token![->]) {
            pattern.push(input.parse()?);
        }

        let _ = input.parse::<Token![->]>()?;
        let content;
        let _ = braced!(content in input);
        let action = content.parse()?; // TokenStream

        Ok(Rule { is_pub, name, return_type, variants: vec![RuleVariant { pattern, action }] })
    }
}

impl Parse for Pattern {
    fn parse(input: ParseStream) -> Result<Self> {
        // Fall 1: String Literal "fn"
        if input.peek(LitStr) {
            return Ok(Pattern::Lit(input.parse()?));
        }

        // Fall 2: Rule Call name:ident()
        // Check binding
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
        // Leer lassen für Stage 0
        
        Ok(Pattern::RuleCall { binding, rule_name, args: vec![] })
    }
}
