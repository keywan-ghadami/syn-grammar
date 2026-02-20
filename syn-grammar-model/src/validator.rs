//! Semantic validation for the grammar model.

use crate::model::*;
use std::collections::{HashMap, HashSet};

pub fn validate<B: Backend>(grammar: &GrammarDefinition) -> syn::Result<()> {
    let builtins = B::get_builtins();
    let builtin_names: HashSet<String> = builtins.iter().map(|b| b.name.to_string()).collect();

    let mut defined_rules = HashSet::new();
    for rule in &grammar.rules {
        if !defined_rules.insert(rule.name.to_string()) {
            return Err(syn::Error::new(
                rule.name.span(),
                format!("Duplicate rule definition: '{}'", rule.name),
            ));
        }
    }

    let all_defs: HashSet<_> = grammar
        .rules
        .iter()
        .map(|r| r.name.to_string())
        .chain(builtin_names.iter().cloned())
        .collect();

    let should_validate_rule_calls = grammar.inherits.is_none();

    if should_validate_rule_calls {
        for rule in &grammar.rules {
            validate_rule(rule, &all_defs)?;
        }
    }

    validate_argument_counts(grammar)?;

    // Perform advanced analysis
    let analysis = crate::analysis::analyze_grammar(grammar);

    // 1. Detect Infinite Recursion (Error)
    for cycle in &analysis.cycles {
        // We filter out self-loops (length 1) because the macro handles direct left recursion (e.g. A -> A b).
        // We only report indirect recursion (A -> B -> A) or complex cycles that are not supported.
        if cycle.len() > 1 {
            let cycle_str = cycle
                .iter()
                .chain(std::iter::once(&cycle[0]))
                .cloned()
                .collect::<Vec<_>>()
                .join(" -> ");
            let msg = format!(
                "Indirect left recursion detected (unsupported): {}",
                cycle_str
            );

            let rule_name = &cycle[0];
            let rule = grammar.rules.iter().find(|r| r.name == *rule_name).unwrap();
            return Err(syn::Error::new(rule.name.span(), msg));
        }
    }

    // 2. Warn about Unused Rules
    if should_validate_rule_calls {
        let mut unused: Vec<_> = analysis.unused_rules.iter().collect();
        unused.sort();
        for rule_name in unused {
            if !rule_name.starts_with('_') {
                eprintln!("warning: Unused rule: '{}'", rule_name);
            }
        }

        // 3. Shadowing / Ambiguity Errors
        if !analysis.errors.is_empty() {
            let mut err = analysis.errors[0].clone();
            for error in analysis.errors.iter().skip(1) {
                err.combine(error.clone());
            }
            return Err(err);
        }
    }

    Ok(())
}

fn validate_rule(rule: &Rule, all_defs: &HashSet<String>) -> syn::Result<()> {
    for variant in &rule.variants {
        validate_pattern_sequence(&variant.pattern, all_defs, &rule.params)?;
    }
    Ok(())
}

fn validate_pattern_sequence(
    patterns: &[ModelPattern],
    all_defs: &HashSet<String>,
    params: &[(syn::Ident, Option<syn::Type>)],
) -> syn::Result<()> {
    for pattern in patterns {
        validate_pattern(pattern, all_defs, params)?;
    }
    Ok(())
}

fn validate_pattern(
    pattern: &ModelPattern,
    all_defs: &HashSet<String>,
    params: &[(syn::Ident, Option<syn::Type>)],
) -> syn::Result<()> {
    match pattern {
        ModelPattern::RuleCall {
            rule_name, args, ..
        } => {
            // Check if rule_name is in all_defs OR in params (as a grammar parameter)
            let is_param = params.iter().any(|(p_name, _)| p_name == rule_name);

            // Special case: separated and repeated are built-ins we are adding logic for,
            // but they might not be in B::get_builtins() if B doesn't declare them.
            // For now, let's assume they are either in builtins or we bypass check for them if generic.
            // Actually, ADR 004 says they are "Built-in Parametric Rules".

            // Note: If 'separated' is not in all_defs, we might error.
            // The backend should probably export them or we hardcode them here?
            // "separated" and "repeated" are portable built-ins.
            let is_portable_builtin = rule_name == "separated" || rule_name == "repeated";

            if !all_defs.contains(&rule_name.to_string()) && !is_param && !is_portable_builtin {
                return Err(syn::Error::new(
                    rule_name.span(),
                    format!("Undefined rule: '{}'", rule_name),
                ));
            }

            for arg in args {
                match arg {
                    Argument::Positional(p) | Argument::Named(_, p) => {
                        validate_pattern(p, all_defs, params)?;
                    }
                }
            }
        }
        ModelPattern::Repeat(inner, _)
        | ModelPattern::Plus(inner, _)
        | ModelPattern::Optional(inner, _)
        | ModelPattern::SpanBinding(inner, _, _)
        | ModelPattern::Peek(inner, _) => {
            validate_pattern(inner, all_defs, params)?;
        }
        ModelPattern::Not(inner, _) => {
            validate_pattern(inner, all_defs, params)?;
        }
        ModelPattern::Group(variants, _) => {
            for seq in variants {
                validate_pattern_sequence(seq, all_defs, params)?;
            }
        }
        ModelPattern::Bracketed(seq, _)
        | ModelPattern::Braced(seq, _)
        | ModelPattern::Parenthesized(seq, _) => {
            validate_pattern_sequence(seq, all_defs, params)?;
        }
        ModelPattern::Recover { body, sync, .. } => {
            validate_pattern(body, all_defs, params)?;
            validate_pattern(sync, all_defs, params)?;
        }
        ModelPattern::Until { pattern, .. } => {
            validate_pattern(pattern, all_defs, params)?;
            validate_no_bindings(pattern)?;
        }
        _ => {}
    }
    Ok(())
}

fn validate_no_bindings(pattern: &ModelPattern) -> syn::Result<()> {
    match pattern {
        ModelPattern::Lit { binding, .. } => {
            if binding.is_some() {
                return Err(syn::Error::new(
                    binding.as_ref().unwrap().span(),
                    "Bindings are not allowed inside 'until' patterns.",
                ));
            }
        }
        ModelPattern::RuleCall { binding, args, .. } => {
            if binding.is_some() {
                return Err(syn::Error::new(
                    binding.as_ref().unwrap().span(),
                    "Bindings are not allowed inside 'until' patterns.",
                ));
            }
            for arg in args {
                match arg {
                    Argument::Positional(p) | Argument::Named(_, p) => {
                        validate_no_bindings(p)?;
                    }
                }
            }
        }
        ModelPattern::Group(variants, _) => {
            for seq in variants {
                for p in seq {
                    validate_no_bindings(p)?;
                }
            }
        }
        ModelPattern::Bracketed(seq, _)
        | ModelPattern::Braced(seq, _)
        | ModelPattern::Parenthesized(seq, _) => {
            for p in seq {
                validate_no_bindings(p)?;
            }
        }
        ModelPattern::Optional(inner, _)
        | ModelPattern::Repeat(inner, _)
        | ModelPattern::Plus(inner, _)
        | ModelPattern::Peek(inner, _)
        | ModelPattern::Not(inner, _)
        | ModelPattern::Until { pattern: inner, .. } => {
            validate_no_bindings(inner)?;
        }
        ModelPattern::SpanBinding(_, ident, _) => {
            return Err(syn::Error::new(
                ident.span(),
                "Span bindings (@) are not allowed inside 'until' patterns.",
            ));
        }
        ModelPattern::Recover {
            binding,
            body,
            sync,
            ..
        } => {
            if binding.is_some() {
                return Err(syn::Error::new(
                    binding.as_ref().unwrap().span(),
                    "Bindings are not allowed inside 'until' patterns.",
                ));
            }
            validate_no_bindings(body)?;
            validate_no_bindings(sync)?;
        }
        ModelPattern::Cut(_) => {}
    }
    Ok(())
}

// Argument count validation
fn validate_argument_counts(grammar: &GrammarDefinition) -> syn::Result<()> {
    let rule_map: HashMap<_, _> = grammar
        .rules
        .iter()
        .map(|r| (r.name.to_string(), r))
        .collect();

    for rule in &grammar.rules {
        for variant in &rule.variants {
            // Recursive validation of arguments
            validate_args_recursive(&variant.pattern, &rule_map)?;
        }
    }
    Ok(())
}

fn validate_args_recursive(
    patterns: &[ModelPattern],
    rule_map: &HashMap<String, &Rule>,
) -> syn::Result<()> {
    for pattern in patterns {
        match pattern {
            ModelPattern::RuleCall {
                rule_name, args, ..
            } => {
                let name_str = rule_name.to_string();

                // Allow named args for specific built-ins or generic checks?
                // For user-defined rules, we currently only support positional args.
                // If we see Named args for user rule, it's an error unless we implement named params for user rules.

                if let Some(target_rule) = rule_map.get(&name_str) {
                    // Check if any args are named
                    for arg in args {
                        if let Argument::Named(n, _) = arg {
                            return Err(syn::Error::new(
                                n.span(),
                                "Named arguments are not supported for user-defined rules yet.",
                            ));
                        }
                    }

                    if target_rule.params.len() != args.len() {
                        return Err(syn::Error::new(
                            rule_name.span(),
                            format!(
                                "Rule '{}' expects {} argument(s), but got {}.",
                                rule_name,
                                target_rule.params.len(),
                                args.len()
                            ),
                        ));
                    }
                } else {
                    // It might be a builtin. We allow arguments for builtins.
                }
                // Recursively check arguments (they are patterns)
                for arg in args {
                    match arg {
                        Argument::Positional(p) | Argument::Named(_, p) => {
                            validate_args_recursive(std::slice::from_ref(p), rule_map)?;
                        }
                    }
                }
            }
            ModelPattern::Repeat(inner, _)
            | ModelPattern::Plus(inner, _)
            | ModelPattern::Optional(inner, _)
            | ModelPattern::SpanBinding(inner, _, _)
            | ModelPattern::Peek(inner, _) => {
                validate_args_recursive(std::slice::from_ref(inner), rule_map)?;
            }
            ModelPattern::Not(inner, _) => {
                validate_args_recursive(std::slice::from_ref(inner), rule_map)?;
            }
            ModelPattern::Group(variants, _) => {
                for seq in variants {
                    validate_args_recursive(seq, rule_map)?;
                }
            }
            ModelPattern::Bracketed(seq, _)
            | ModelPattern::Braced(seq, _)
            | ModelPattern::Parenthesized(seq, _) => {
                validate_args_recursive(seq, rule_map)?;
            }
            ModelPattern::Recover { body, sync, .. } => {
                validate_args_recursive(std::slice::from_ref(body), rule_map)?;
                validate_args_recursive(std::slice::from_ref(sync), rule_map)?;
            }
            ModelPattern::Until { pattern, .. } => {
                validate_args_recursive(std::slice::from_ref(pattern), rule_map)?;
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    struct TestBackend;
    impl Backend for TestBackend {
        fn get_builtins() -> &'static [BuiltIn] {
            &[
                BuiltIn {
                    name: "ident",
                    return_type: "syn::Ident",
                },
                BuiltIn {
                    name: "string",
                    return_type: "String",
                },
            ]
        }
    }

    fn parse_model(input: proc_macro2::TokenStream) -> GrammarDefinition {
        let p_ast: crate::parser::GrammarDefinition = syn::parse2(input).unwrap();
        p_ast.into()
    }

    #[test]
    fn test_undefined_rule() {
        let input = quote! {
            grammar test {
                rule main -> () = undefined_rule -> { () }
            }
        };
        let model = parse_model(input);
        let err = validate::<TestBackend>(&model);
        match err {
            Ok(_) => panic!("Expected undefined rule error"),
            Err(e) => assert_eq!(e.to_string(), "Undefined rule: 'undefined_rule'"),
        }
    }

    #[test]
    fn test_duplicate_rule() {
        let input = quote! {
            grammar test {
                rule main -> () = "a" -> { () }
                rule main -> () = "b" -> { () }
            }
        };
        let model = parse_model(input);
        let err = validate::<TestBackend>(&model).unwrap_err();
        assert_eq!(err.to_string(), "Duplicate rule definition: 'main'");
    }

    #[test]
    fn test_rule_args_mismatch() {
        let input = quote! {
            grammar test {
                rule main -> () = sub(1) -> { () }
                rule sub -> () = "hello" -> { () }
            }
        };
        let model = parse_model(input);

        let expected_span = model.rules[0].variants[0].pattern[0].span();

        let err = validate::<TestBackend>(&model).unwrap_err();
        assert_eq!(
            err.to_string(),
            "Rule 'sub' expects 0 argument(s), but got 1."
        );
        assert_eq!(format!("{:?}", err.span()), format!("{:?}", expected_span));
    }

    #[test]
    fn test_shadowing_identical() {
        let input = quote! {
            grammar test {
                rule main -> ()
                    = "a" -> { () }
                    | "a" -> { () }
            }
        };
        let model = parse_model(input);
        let err = validate::<TestBackend>(&model).unwrap_err();
        assert!(err
            .to_string()
            .contains("Alternative 1 and 2 are identical"));
    }

    #[test]
    fn test_shadowing_prefix() {
        let input = quote! {
            grammar test {
                rule main -> ()
                    = "a" -> { () }
                    | "a" "b" -> { () }
            }
        };
        let model = parse_model(input);
        let err = validate::<TestBackend>(&model).unwrap_err();
        assert!(err
            .to_string()
            .contains("Alternative 1 shadows Alternative 2"));
    }

    #[test]
    fn test_no_shadowing() {
        let input = quote! {
            grammar test {
                rule main -> ()
                    = "a" "b" -> { () }
                    | "a" -> { () }
            }
        };
        let model = parse_model(input);
        validate::<TestBackend>(&model).unwrap();
    }

    #[test]
    fn test_bug_typed_param() {
        let input = quote! {
            grammar test {
                rule list<T>(item: Type) -> () = item -> { () }
            }
        };
        let model = parse_model(input);
        // This fails in 0.7.0 with "Undefined rule: 'item'"
        validate::<TestBackend>(&model).expect("Validation failed for typed parameter");
    }

    #[test]
    fn test_until_binding_fail() {
        let input = quote! {
            grammar test {
                rule main -> () = until(x: "a") -> { () }
            }
        };
        let model = parse_model(input);
        let err = validate::<TestBackend>(&model).unwrap_err();
        assert_eq!(
            err.to_string(),
            "Bindings are not allowed inside 'until' patterns."
        );
    }
}
