use crate::model::*;
use syn::{Result, Error};
use std::collections::HashMap;

pub fn validate(grammar: &GrammarDefinition) -> Result<()> {
    let mut defined_rules = HashMap::new();
    
    // 1. Collect all defined rules and their parameter count
    for rule in &grammar.rules {
        defined_rules.insert(rule.name.to_string(), rule.params.len());
    }

    // 2. Validate all rule calls inside patterns
    for rule in &grammar.rules {
        for variant in &rule.variants {
            validate_patterns(&variant.pattern, &defined_rules, grammar.inherits.is_some())?;
        }
    }

    Ok(())
}

fn validate_patterns(
    patterns: &[ModelPattern], 
    defined_rules: &HashMap<String, usize>,
    has_inheritance: bool
) -> Result<()> {
    for pattern in patterns {
        match pattern {
            ModelPattern::RuleCall { rule_name, args, .. } => {
                let name_str = rule_name.to_string();
                
                if is_builtin(&name_str) {
                    // Builtins usually take 0 args, except maybe special ones in future.
                    // For now, we assume builtins like 'ident', 'int_lit' take 0 args.
                    if !args.is_empty() {
                        return Err(Error::new(rule_name.span(), format!("Built-in rule '{}' does not accept arguments.", name_str)));
                    }
                } else if let Some(&param_count) = defined_rules.get(&name_str) {
                    // Local rule found: Check argument count
                    if args.len() != param_count {
                        return Err(Error::new(rule_name.span(), 
                            format!("Rule '{}' expects {} argument(s), but got {}.", name_str, param_count, args.len())));
                    }
                } else {
                    // Rule not found locally.
                    // If we have inheritance, it might be in the parent grammar (we can't check that easily here).
                    // If no inheritance, it's definitely an error.
                    if !has_inheritance {
                        return Err(Error::new(rule_name.span(), format!("Undefined rule: '{}'.", name_str)));
                    }
                }
            },
            ModelPattern::Group(alts) => {
                for alt in alts {
                    validate_patterns(alt, defined_rules, has_inheritance)?;
                }
            },
            ModelPattern::Optional(p) | ModelPattern::Repeat(p) | ModelPattern::Plus(p) => {
                validate_patterns(std::slice::from_ref(p), defined_rules, has_inheritance)?;
            },
            ModelPattern::Bracketed(p) | ModelPattern::Braced(p) | ModelPattern::Parenthesized(p) => {
                validate_patterns(p, defined_rules, has_inheritance)?;
            },
            _ => {}
        }
    }
    Ok(())
}

fn is_builtin(name: &str) -> bool {
    matches!(name, "ident" | "int_lit" | "string_lit" | "rust_type" | "rust_block" | "lit_str")
}
