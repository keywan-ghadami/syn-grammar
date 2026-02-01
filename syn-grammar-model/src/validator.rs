// Moved from macros/src/validator.rs
use crate::model::*;
use syn::{Result, Error};
use std::collections::HashMap;

/// Validates the semantic model of the grammar.
///
/// Checks for:
/// - Undefined rules.
/// - Argument count mismatches in rule calls.
/// - Invalid usage of built-in rules.
///
/// Returns `Ok(())` if the grammar is valid, or a `syn::Error` pointing to the location of the issue.
pub fn validate(grammar: &GrammarDefinition, valid_builtins: &[&str]) -> Result<()> {
    let mut defined_rules = HashMap::new();
    
    for rule in &grammar.rules {
        defined_rules.insert(rule.name.to_string(), rule.params.len());
    }

    for rule in &grammar.rules {
        for variant in &rule.variants {
            validate_patterns(&variant.pattern, &defined_rules, grammar.inherits.is_some(), valid_builtins)?;
        }
    }

    Ok(())
}

fn validate_patterns(
    patterns: &[ModelPattern], 
    defined_rules: &HashMap<String, usize>,
    has_inheritance: bool,
    valid_builtins: &[&str]
) -> Result<()> {
    for pattern in patterns {
        match pattern {
            ModelPattern::RuleCall { rule_name, args, .. } => {
                let name_str = rule_name.to_string();
                
                if valid_builtins.contains(&name_str.as_str()) {
                    if !args.is_empty() {
                        return Err(Error::new(rule_name.span(), format!("Built-in rule '{}' does not accept arguments.", name_str)));
                    }
                } else if let Some(&param_count) = defined_rules.get(&name_str) {
                    if args.len() != param_count {
                        return Err(Error::new(rule_name.span(), 
                            format!("Rule '{}' expects {} argument(s), but got {}.", name_str, param_count, args.len())));
                    }
                } else if !has_inheritance {
                    return Err(Error::new(rule_name.span(), format!("Undefined rule: '{}'.", name_str)));
                }
            },
            ModelPattern::Group(alts) => {
                for alt in alts {
                    validate_patterns(alt, defined_rules, has_inheritance, valid_builtins)?;
                }
            },
            ModelPattern::Optional(p) | ModelPattern::Repeat(p) | ModelPattern::Plus(p) => {
                validate_patterns(std::slice::from_ref(p), defined_rules, has_inheritance, valid_builtins)?;
            },
            ModelPattern::Bracketed(p) | ModelPattern::Braced(p) | ModelPattern::Parenthesized(p) => {
                validate_patterns(p, defined_rules, has_inheritance, valid_builtins)?;
            },
            ModelPattern::Recover { body, sync, .. } => {
                validate_patterns(std::slice::from_ref(body), defined_rules, has_inheritance, valid_builtins)?;
                validate_patterns(std::slice::from_ref(sync), defined_rules, has_inheritance, valid_builtins)?;
            },
            _ => {}
        }
    }
    Ok(())
}
