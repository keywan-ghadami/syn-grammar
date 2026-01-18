fn generate_variants(variants: &[RuleVariant], kws: &HashSet<String>) -> Result<TokenStream> {
    let mut chain = quote! { Err(input.error("No variant matched")) };

    for variant in variants.iter().rev() {
        let logic = generate_sequence(&variant.pattern, &variant.action, kws)?;
        chain = quote! {
            if let Some(res) = rt::attempt(input, |input| { #logic })? {
                Ok(res)
            } else {
                #chain
            }
        };
    }
    Ok(chain)
}
