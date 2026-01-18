fn generate_variants(variants: &[RuleVariant], is_top_level: bool, kws: &HashSet<String>) -> Result<TokenStream> {
    let mut current_code = quote! { Err(input.error("No match")) };

    for (i, variant) in variants.iter().enumerate().rev() {
        let logic = generate_sequence(&variant.pattern, &variant.action, kws)?;
        
        // Nutzt jetzt die Runtime-Speculation für alles, was kein simpler Peek ist
        current_code = quote! {
            if let Some(res) = rt::parse_try(input, |input| { #logic })? {
                res
            } else {
                #current_code
            }
        };
    }
    Ok(current_code)
}

