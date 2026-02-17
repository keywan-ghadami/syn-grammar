use crate::model::*;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use std::collections::{HashMap, HashSet, VecDeque};
use syn::{parse_quote, Lit, Result};

/// Collects all custom keywords from the grammar
pub fn collect_custom_keywords(grammar: &GrammarDefinition) -> HashSet<String> {
    let mut kws = HashSet::new();
    grammar
        .rules
        .iter()
        .flat_map(|r| &r.variants)
        .for_each(|v| collect_from_patterns(&v.pattern, &mut kws));
    kws
}

/// Result of analyzing a pattern sequence for a Cut operator (`=>`)
pub struct CutAnalysis<'a> {
    pub pre_cut: &'a [ModelPattern],
    pub post_cut: &'a [ModelPattern],
}

/// Checks if a sequence contains a Cut operator and splits it.
pub fn find_cut<'a>(patterns: &'a [ModelPattern]) -> Option<CutAnalysis<'a>> {
    let idx = patterns
        .iter()
        .position(|p| matches!(p, ModelPattern::Cut(_)))?;
    Some(CutAnalysis {
        pre_cut: &patterns[0..idx],
        post_cut: &patterns[idx + 1..],
    })
}

/// Splits variants into recursive (starts with the rule name) and base cases.
pub fn split_left_recursive<'a>(
    rule_name: &Ident,
    variants: &'a [RuleVariant],
) -> (Vec<&'a RuleVariant>, Vec<&'a RuleVariant>) {
    let mut recursive = Vec::new();
    let mut base = Vec::new();

    for v in variants {
        if let Some(ModelPattern::RuleCall { rule_name: r, .. }) = v.pattern.first() {
            if r == rule_name {
                recursive.push(v);
                continue;
            }
        }
        base.push(v);
    }
    (recursive, base)
}

fn collect_from_patterns(patterns: &[ModelPattern], kws: &mut HashSet<String>) {
    for p in patterns {
        match p {
            ModelPattern::Lit(Lit::Str(lit)) => {
                let s = lit.value();
                // Try to tokenize the string literal to find identifiers
                if let Ok(ts) = syn::parse_str::<proc_macro2::TokenStream>(&s) {
                    for token in ts {
                        if let proc_macro2::TokenTree::Ident(ident) = token {
                            let s = ident.to_string();
                            // If syn accepts it as an Ident, it's a candidate.
                            // We rely on syn::parse_str::<syn::Ident> to filter out reserved keywords.
                            // We also exclude "_" because it cannot be a struct name for custom_keyword!.
                            if s != "_" && syn::parse_str::<syn::Ident>(&s).is_ok() {
                                kws.insert(s);
                            }
                        }
                    }
                }
            }
            ModelPattern::Group(alts, _) => {
                alts.iter().for_each(|alt| collect_from_patterns(alt, kws))
            }
            ModelPattern::Bracketed(s, _)
            | ModelPattern::Braced(s, _)
            | ModelPattern::Parenthesized(s, _) => collect_from_patterns(s, kws),
            ModelPattern::Optional(i, _)
            | ModelPattern::Repeat(i, _)
            | ModelPattern::Plus(i, _) => collect_from_patterns(std::slice::from_ref(i), kws),
            ModelPattern::SpanBinding(i, _, _) => {
                collect_from_patterns(std::slice::from_ref(i), kws)
            }
            ModelPattern::Recover { body, sync, .. } => {
                collect_from_patterns(std::slice::from_ref(body), kws);
                collect_from_patterns(std::slice::from_ref(sync), kws);
            }
            ModelPattern::Peek(i, _) | ModelPattern::Not(i, _) => {
                collect_from_patterns(std::slice::from_ref(i), kws)
            }
            _ => {}
        }
    }
}

pub fn collect_bindings(patterns: &[ModelPattern]) -> Vec<Ident> {
    let mut bindings = Vec::new();
    for p in patterns {
        match p {
            ModelPattern::RuleCall {
                binding: Some(b), ..
            } => bindings.push(b.clone()),
            ModelPattern::Repeat(inner, _)
            | ModelPattern::Plus(inner, _)
            | ModelPattern::Optional(inner, _) => {
                bindings.extend(collect_bindings(std::slice::from_ref(inner)));
            }
            ModelPattern::Parenthesized(s, _)
            | ModelPattern::Bracketed(s, _)
            | ModelPattern::Braced(s, _) => {
                bindings.extend(collect_bindings(s));
            }
            ModelPattern::SpanBinding(inner, ident, _) => {
                bindings.push(ident.clone());
                bindings.extend(collect_bindings(std::slice::from_ref(inner)));
            }
            ModelPattern::Recover { binding, body, .. } => {
                if let Some(b) = binding {
                    bindings.push(b.clone());
                } else {
                    bindings.extend(collect_bindings(std::slice::from_ref(body)));
                }
            }
            ModelPattern::Peek(inner, _) => {
                bindings.extend(collect_bindings(std::slice::from_ref(inner)));
            }
            ModelPattern::Group(alts, _) => {
                for alt in alts {
                    bindings.extend(collect_bindings(alt));
                }
            }
            ModelPattern::Not(_, _) => {
                // Not(...) bindings are ignored/dropped because it only succeeds if inner fails.
            }
            _ => {}
        }
    }
    bindings
}

/// Returns the sequence of tokens for syn::parse::<Token>()
pub fn resolve_token_types(
    lit: &syn::LitStr,
    custom_keywords: &HashSet<String>,
) -> Result<Vec<syn::Type>> {
    let s = lit.value();

    if custom_keywords.contains(&s) {
        let ident = format_ident!("{}", s);
        return Ok(vec![parse_quote!(kw::#ident)]);
    }

    if matches!(s.as_str(), "(" | ")" | "[" | "]" | "{" | "}") {
        return Err(syn::Error::new(
            lit.span(),
            format!(
                "Invalid direct token literal: '{}'. Use paren(...), [...] or {{...}} instead.",
                s
            ),
        ));
    }

    if s == "true" || s == "false" {
        return Err(syn::Error::new(
            lit.span(),
            format!(
                "Boolean literal '{}' cannot be used as a token. Use `lit_bool` parser instead.",
                s
            ),
        ));
    }
    if s.chars().next().is_some_and(|c| c.is_numeric()) {
        return Err(syn::Error::new(lit.span(),
            format!("Numeric literal '{}' cannot be used as a token. Use `integer` or `lit_int` parsers instead.", s)));
    }

    let ts: proc_macro2::TokenStream = syn::parse_str(&s)
        .map_err(|_| syn::Error::new(lit.span(), format!("Invalid token literal: '{}'", s)))?;

    let mut types = Vec::new();
    for token in ts {
        match token {
            proc_macro2::TokenTree::Punct(p) => {
                let c = p.as_char();
                let ty: syn::Type = syn::parse_str(&format!("Token![{}]", c)).map_err(|_| {
                    syn::Error::new(
                        lit.span(),
                        format!("Cannot map punctuation '{}' to Token!", c),
                    )
                })?;
                types.push(ty);
            }
            proc_macro2::TokenTree::Ident(i) => {
                let s = i.to_string();
                if custom_keywords.contains(&s) {
                    let ident = format_ident!("{}", s);
                    types.push(parse_quote!(kw::#ident));
                } else {
                    let ty: syn::Type =
                        syn::parse_str(&format!("Token![{}]", s)).map_err(|_| {
                            syn::Error::new(
                                lit.span(),
                                format!(
                                "Identifier '{}' is not a custom keyword and not a valid Token!",
                                s
                            ),
                            )
                        })?;
                    types.push(ty);
                }
            }
            _ => {
                return Err(syn::Error::new(
                    lit.span(),
                    "Literal contains unsupported token tree (Group or Literal)",
                ))
            }
        }
    }

    if types.is_empty() {
        return Err(syn::Error::new(
            lit.span(),
            "Empty string literal is not supported.",
        ));
    }

    Ok(types)
}

/// Helper for UPO: Returns a TokenStream for input.peek(...)
pub fn get_simple_peek(
    pattern: &ModelPattern,
    kws: &HashSet<String>,
) -> Result<Option<TokenStream>> {
    match pattern {
        ModelPattern::Lit(Lit::Str(lit)) => {
            let token_types = resolve_token_types(lit, kws)?;
            if let Some(first_type) = token_types.first() {
                Ok(Some(quote!(#first_type)))
            } else {
                Ok(None)
            }
        }
        ModelPattern::Lit(_) => Ok(None),
        ModelPattern::Bracketed(_, _) => Ok(Some(quote!(syn::token::Bracket))),
        ModelPattern::Braced(_, _) => Ok(Some(quote!(syn::token::Brace))),
        ModelPattern::Parenthesized(_, _) => Ok(Some(quote!(syn::token::Paren))),
        ModelPattern::Optional(inner, _)
        | ModelPattern::Repeat(inner, _)
        | ModelPattern::Plus(inner, _) => get_simple_peek(inner, kws),
        ModelPattern::SpanBinding(inner, _, _) => get_simple_peek(inner, kws),
        ModelPattern::Recover { body, .. } => get_simple_peek(body, kws),
        ModelPattern::Group(alts, _) => {
            if alts.len() == 1 {
                if let Some(first) = alts[0].first() {
                    get_simple_peek(first, kws)
                } else {
                    Ok(None)
                }
            } else {
                Ok(None)
            }
        }
        ModelPattern::Peek(inner, _) => get_simple_peek(inner, kws),
        ModelPattern::Not(_, _) => Ok(None),
        _ => Ok(None),
    }
}

/// Helper for UPO: Returns a unique string key for the start token
pub fn get_peek_token_string(patterns: &[ModelPattern]) -> Option<String> {
    match patterns.first() {
        Some(ModelPattern::Lit(Lit::Str(l))) => Some(l.value()),
        Some(ModelPattern::Lit(_)) => None,
        Some(ModelPattern::Bracketed(_, _)) => Some("Bracket".to_string()),
        Some(ModelPattern::Braced(_, _)) => Some("Brace".to_string()),
        Some(ModelPattern::Parenthesized(_, _)) => Some("Paren".to_string()),
        Some(ModelPattern::Optional(inner, _))
        | Some(ModelPattern::Repeat(inner, _))
        | Some(ModelPattern::Plus(inner, _)) => {
            get_peek_token_string(std::slice::from_ref(&**inner))
        }
        Some(ModelPattern::SpanBinding(inner, _, _)) => {
            get_peek_token_string(std::slice::from_ref(&**inner))
        }
        Some(ModelPattern::Recover { body, .. }) => {
            get_peek_token_string(std::slice::from_ref(&**body))
        }
        Some(ModelPattern::Group(alts, _)) => {
            if alts.len() == 1 {
                get_peek_token_string(&alts[0])
            } else {
                None
            }
        }
        Some(ModelPattern::Peek(inner, _)) => get_peek_token_string(std::slice::from_ref(&**inner)),
        Some(ModelPattern::Not(_, _)) => None,
        _ => None,
    }
}

pub fn is_nullable(pattern: &ModelPattern) -> bool {
    match pattern {
        ModelPattern::Cut(_) => true,
        ModelPattern::Lit(_) => false,
        ModelPattern::RuleCall { .. } => true,
        ModelPattern::Group(alts, _) => alts.iter().any(|seq| seq.iter().all(is_nullable)),
        ModelPattern::Bracketed(_, _)
        | ModelPattern::Braced(_, _)
        | ModelPattern::Parenthesized(_, _) => false,
        ModelPattern::Optional(_, _) => true,
        ModelPattern::Repeat(_, _) => true,
        ModelPattern::Plus(inner, _) => is_nullable(inner),
        ModelPattern::SpanBinding(inner, _, _) => is_nullable(inner),
        ModelPattern::Recover { .. } => true,
        ModelPattern::Peek(_, _) => true,
        ModelPattern::Not(_, _) => true,
    }
}

// ==============================================================================
//  Graph Analysis & Diagnostics (Infinite Recursion, Ambiguity, Unused Rules)
// ==============================================================================

pub struct GrammarAnalysis {
    pub nullable_rules: HashSet<String>,
    pub cycles: Vec<Vec<String>>,
    pub unused_rules: HashSet<String>,
    pub first_sets: HashMap<String, HashSet<String>>,
    pub errors: Vec<syn::Error>,
}

pub fn analyze_grammar(grammar: &GrammarDefinition) -> GrammarAnalysis {
    let mut nullable_rules = HashSet::new();

    // 1. Compute Nullable Fixpoint
    let mut changed = true;
    while changed {
        changed = false;
        for rule in &grammar.rules {
            let rule_name = rule.name.to_string();
            if nullable_rules.contains(&rule_name) {
                continue;
            }

            let mut is_rule_nullable = false;
            for variant in &rule.variants {
                if is_sequence_nullable(&variant.pattern, &nullable_rules) {
                    is_rule_nullable = true;
                    break;
                }
            }

            if is_rule_nullable {
                nullable_rules.insert(rule_name);
                changed = true;
            }
        }
    }

    // 2. Detect Cycles
    let cycles = find_cycles(grammar, &nullable_rules);

    // 3. Unused Rules
    let unused_rules = find_unused_rules(grammar);

    // 4. FIRST sets and Errors
    let (first_sets, errors) = compute_first_sets_and_errors(grammar, &nullable_rules);

    GrammarAnalysis {
        nullable_rules,
        cycles,
        unused_rules,
        first_sets,
        errors,
    }
}

fn is_sequence_nullable(patterns: &[ModelPattern], nullable_rules: &HashSet<String>) -> bool {
    for p in patterns {
        if !is_pattern_nullable_precise(p, nullable_rules) {
            return false;
        }
    }
    true
}

fn is_pattern_nullable_precise(pattern: &ModelPattern, nullable_rules: &HashSet<String>) -> bool {
    match pattern {
        ModelPattern::Cut(_) => true,
        ModelPattern::Lit(_) => false,
        ModelPattern::RuleCall { rule_name, .. } => nullable_rules.contains(&rule_name.to_string()),
        ModelPattern::Group(alts, _) => alts
            .iter()
            .any(|seq| is_sequence_nullable(seq, nullable_rules)),
        ModelPattern::Optional(_, _)
        | ModelPattern::Repeat(_, _)
        | ModelPattern::Recover { .. }
        | ModelPattern::Peek(_, _)
        | ModelPattern::Not(_, _) => true, // Peek/Not consume nothing
        ModelPattern::Plus(inner, _) => is_pattern_nullable_precise(inner, nullable_rules),
        ModelPattern::SpanBinding(inner, _, _) => {
            is_pattern_nullable_precise(inner, nullable_rules)
        }
        ModelPattern::Bracketed(_, _)
        | ModelPattern::Braced(_, _)
        | ModelPattern::Parenthesized(_, _) => false,
    }
}

fn find_cycles(grammar: &GrammarDefinition, nullable_rules: &HashSet<String>) -> Vec<Vec<String>> {
    let mut adj = HashMap::new();
    for rule in &grammar.rules {
        let mut deps = HashSet::new();
        for variant in &rule.variants {
            collect_nullable_deps(&variant.pattern, nullable_rules, &mut deps);
        }
        adj.insert(rule.name.to_string(), deps);
    }

    let mut cycles = Vec::new();
    let mut visited = HashSet::new();
    let mut stack = Vec::new();
    let mut on_stack = HashSet::new();

    for rule in &grammar.rules {
        let name = rule.name.to_string();
        if !visited.contains(&name) {
            find_cycles_dfs(
                &name,
                &adj,
                &mut visited,
                &mut stack,
                &mut on_stack,
                &mut cycles,
            );
        }
    }
    cycles
}

fn find_cycles_dfs(
    u: &String,
    adj: &HashMap<String, HashSet<String>>,
    visited: &mut HashSet<String>,
    stack: &mut Vec<String>,
    on_stack: &mut HashSet<String>,
    cycles: &mut Vec<Vec<String>>,
) {
    visited.insert(u.clone());
    stack.push(u.clone());
    on_stack.insert(u.clone());

    if let Some(neighbors) = adj.get(u) {
        for v in neighbors {
            if on_stack.contains(v) {
                // Found cycle: v ... u -> v
                if let Some(pos) = stack.iter().position(|x| x == v) {
                    cycles.push(stack[pos..].to_vec());
                }
            } else if !visited.contains(v) {
                find_cycles_dfs(v, adj, visited, stack, on_stack, cycles);
            }
        }
    }

    on_stack.remove(u);
    stack.pop();
}

fn collect_nullable_deps(
    patterns: &[ModelPattern],
    nullable_rules: &HashSet<String>,
    deps: &mut HashSet<String>,
) {
    for p in patterns {
        match p {
            ModelPattern::RuleCall { rule_name, .. } => {
                deps.insert(rule_name.to_string());
                if !nullable_rules.contains(&rule_name.to_string()) {
                    return;
                }
            }
            ModelPattern::Group(alts, _) => {
                let mut group_nullable = false;
                for alt in alts {
                    collect_nullable_deps(alt, nullable_rules, deps);
                    if is_sequence_nullable(alt, nullable_rules) {
                        group_nullable = true;
                    }
                }
                if !group_nullable {
                    return;
                }
            }
            ModelPattern::Optional(inner, _) | ModelPattern::Repeat(inner, _) => {
                collect_nullable_deps(std::slice::from_ref(inner), nullable_rules, deps);
            }
            ModelPattern::Plus(inner, _) => {
                collect_nullable_deps(std::slice::from_ref(inner), nullable_rules, deps);
                if !is_pattern_nullable_precise(inner, nullable_rules) {
                    return;
                }
            }
            ModelPattern::SpanBinding(inner, _, _) => {
                collect_nullable_deps(std::slice::from_ref(inner), nullable_rules, deps);
                if !is_pattern_nullable_precise(inner, nullable_rules) {
                    return;
                }
            }
            ModelPattern::Peek(inner, _) | ModelPattern::Not(inner, _) => {
                collect_nullable_deps(std::slice::from_ref(inner), nullable_rules, deps);
                // Peek/Not consume nothing, so we continue to next pattern
            }
            ModelPattern::Recover { body, .. } => {
                collect_nullable_deps(std::slice::from_ref(body), nullable_rules, deps);
            }
            ModelPattern::Lit(_)
            | ModelPattern::Bracketed(..)
            | ModelPattern::Braced(..)
            | ModelPattern::Parenthesized(..) => {
                return;
            }
            ModelPattern::Cut(_) => {}
        }
    }
}

fn find_unused_rules(grammar: &GrammarDefinition) -> HashSet<String> {
    let mut used = HashSet::new();
    let mut queue = VecDeque::new();

    for rule in &grammar.rules {
        if rule.is_pub {
            used.insert(rule.name.to_string());
            queue.push_back(rule.name.to_string());
        }
    }

    if used.is_empty() && !grammar.rules.is_empty() {
        let first = grammar.rules[0].name.to_string();
        used.insert(first.clone());
        queue.push_back(first);
    }

    let rule_map: HashMap<_, _> = grammar
        .rules
        .iter()
        .map(|r| (r.name.to_string(), r))
        .collect();

    while let Some(current_name) = queue.pop_front() {
        if let Some(rule) = rule_map.get(&current_name) {
            for variant in &rule.variants {
                collect_called_rules(&variant.pattern, &mut |callee| {
                    if used.insert(callee.clone()) {
                        queue.push_back(callee);
                    }
                });
            }
        }
    }

    grammar
        .rules
        .iter()
        .map(|r| r.name.to_string())
        .filter(|n| !used.contains(n))
        .collect()
}

fn collect_called_rules<F: FnMut(String)>(patterns: &[ModelPattern], cb: &mut F) {
    for p in patterns {
        match p {
            ModelPattern::RuleCall { rule_name, .. } => cb(rule_name.to_string()),
            ModelPattern::Group(alts, _) => {
                for alt in alts {
                    collect_called_rules(alt, cb);
                }
            }
            ModelPattern::Optional(inner, _)
            | ModelPattern::Repeat(inner, _)
            | ModelPattern::Plus(inner, _)
            | ModelPattern::SpanBinding(inner, _, _)
            | ModelPattern::Peek(inner, _)
            | ModelPattern::Not(inner, _) => {
                collect_called_rules(std::slice::from_ref(inner), cb);
            }
            ModelPattern::Recover { body, sync, .. } => {
                collect_called_rules(std::slice::from_ref(body), cb);
                collect_called_rules(std::slice::from_ref(sync), cb);
            }
            ModelPattern::Bracketed(inner, _)
            | ModelPattern::Braced(inner, _)
            | ModelPattern::Parenthesized(inner, _) => {
                collect_called_rules(inner, cb);
            }
            _ => {}
        }
    }
}

fn compute_first_sets_and_errors(
    grammar: &GrammarDefinition,
    nullable_rules: &HashSet<String>,
) -> (HashMap<String, HashSet<String>>, Vec<syn::Error>) {
    let mut first_sets: HashMap<String, HashSet<String>> = HashMap::new();
    let mut errors = Vec::new();

    for rule in &grammar.rules {
        first_sets.insert(rule.name.to_string(), HashSet::new());
    }

    // 1. Compute Nullable Fixpoint for FIRST sets
    let mut changed = true;
    while changed {
        changed = false;
        for rule in &grammar.rules {
            let name = rule.name.to_string();
            let mut current_first = first_sets.get(&name).cloned().unwrap_or_default();
            let start_len = current_first.len();

            for variant in &rule.variants {
                collect_first_from_sequence(
                    &variant.pattern,
                    &first_sets,
                    nullable_rules,
                    &mut current_first,
                );
            }

            if current_first.len() != start_len {
                first_sets.insert(name, current_first);
                changed = true;
            }
        }
    }

    // 2. Generate Shadowing Errors (Exact Duplicate and Prefix Shadowing)
    for rule in &grammar.rules {
        for (i, v1) in rule.variants.iter().enumerate() {
            // Check against subsequent variants
            for (j, v2) in rule.variants.iter().enumerate().skip(i + 1) {
                // Determine span for error reporting
                let span = if let Some(first_pat) = v2.pattern.first() {
                    first_pat.span()
                } else {
                    rule.name.span()
                };

                if sequence_structure_eq(&v1.pattern, &v2.pattern) {
                    errors.push(syn::Error::new(
                        span,
                        format!(
                            "Rule '{}': Alternative {} and {} are identical. Alternative {} is dead code.",
                            rule.name,
                            i + 1,
                            j + 1,
                            j + 1
                        )
                    ));
                    continue; // No need to check prefix if identical
                }

                if sequence_is_prefix(&v1.pattern, &v2.pattern) {
                    errors.push(syn::Error::new(
                        span,
                        format!(
                            "Rule '{}': Alternative {} shadows Alternative {} (prefix). Swap the order for longest-match.",
                            rule.name,
                            i + 1,
                            j + 1
                        )
                    ));
                }
            }
        }
    }

    (first_sets, errors)
}

fn collect_first_from_sequence(
    patterns: &[ModelPattern],
    first_sets: &HashMap<String, HashSet<String>>,
    nullable_rules: &HashSet<String>,
    acc: &mut HashSet<String>,
) {
    for p in patterns {
        match p {
            ModelPattern::Lit(Lit::Str(s)) => {
                acc.insert(format!("\"{}\"", s.value()));
                return;
            }
            ModelPattern::Lit(_) => {
                acc.insert("LIT".to_string());
                return;
            }
            ModelPattern::RuleCall { rule_name, .. } => {
                let name = rule_name.to_string();
                if let Some(fs) = first_sets.get(&name) {
                    acc.extend(fs.clone());
                } else {
                    acc.insert(format!("<{}>", name));
                }
                if !nullable_rules.contains(&name) {
                    return;
                }
            }
            ModelPattern::Group(alts, _) => {
                let mut group_nullable = false;
                for alt in alts {
                    collect_first_from_sequence(alt, first_sets, nullable_rules, acc);
                    if is_sequence_nullable(alt, nullable_rules) {
                        group_nullable = true;
                    }
                }
                if !group_nullable {
                    return;
                }
            }
            ModelPattern::Bracketed(..) => {
                acc.insert("Bracket".to_string());
                return;
            }
            ModelPattern::Braced(..) => {
                acc.insert("Brace".to_string());
                return;
            }
            ModelPattern::Parenthesized(..) => {
                acc.insert("Paren".to_string());
                return;
            }

            ModelPattern::Optional(inner, _)
            | ModelPattern::Repeat(inner, _)
            | ModelPattern::Peek(inner, _) => {
                collect_first_from_sequence(
                    std::slice::from_ref(inner),
                    first_sets,
                    nullable_rules,
                    acc,
                );
                continue;
            }
            ModelPattern::Plus(inner, _) => {
                collect_first_from_sequence(
                    std::slice::from_ref(inner),
                    first_sets,
                    nullable_rules,
                    acc,
                );
                if !is_pattern_nullable_precise(inner, nullable_rules) {
                    return;
                }
            }
            ModelPattern::SpanBinding(inner, _, _) => {
                collect_first_from_sequence(
                    std::slice::from_ref(inner),
                    first_sets,
                    nullable_rules,
                    acc,
                );
                if !is_pattern_nullable_precise(inner, nullable_rules) {
                    return;
                }
            }
            ModelPattern::Not(_inner, _) => {
                continue;
            }
            ModelPattern::Recover { body, .. } => {
                collect_first_from_sequence(
                    std::slice::from_ref(body),
                    first_sets,
                    nullable_rules,
                    acc,
                );
            }
            _ => {}
        }
    }
}

// ==============================================================================
//  Shadowing / Dead Code Analysis Helpers
// ==============================================================================

fn peel(p: &ModelPattern) -> &ModelPattern {
    match p {
        ModelPattern::SpanBinding(inner, _, _) => peel(inner),
        _ => p,
    }
}

fn sequence_structure_eq(seq1: &[ModelPattern], seq2: &[ModelPattern]) -> bool {
    if seq1.len() != seq2.len() {
        return false;
    }
    seq1.iter()
        .zip(seq2.iter())
        .all(|(p1, p2)| pattern_structure_eq(p1, p2))
}

fn sequence_is_prefix(prefix: &[ModelPattern], full: &[ModelPattern]) -> bool {
    if prefix.len() >= full.len() {
        return false;
    }
    prefix
        .iter()
        .zip(full.iter())
        .all(|(p1, p2)| pattern_structure_eq(p1, p2))
}

fn pattern_structure_eq(p1: &ModelPattern, p2: &ModelPattern) -> bool {
    let p1 = peel(p1);
    let p2 = peel(p2);

    match (p1, p2) {
        (ModelPattern::Cut(_), ModelPattern::Cut(_)) => true,
        (ModelPattern::Lit(l1), ModelPattern::Lit(l2)) => l1 == l2,
        (
            ModelPattern::RuleCall {
                rule_name: r1,
                args: a1,
                ..
            },
            ModelPattern::RuleCall {
                rule_name: r2,
                args: a2,
                ..
            },
        ) => r1 == r2 && sequence_structure_eq(a1, a2),
        (ModelPattern::Group(g1, _), ModelPattern::Group(g2, _)) => {
            if g1.len() != g2.len() {
                return false;
            }
            g1.iter()
                .zip(g2.iter())
                .all(|(s1, s2)| sequence_structure_eq(s1, s2))
        }
        (ModelPattern::Bracketed(inner1, _), ModelPattern::Bracketed(inner2, _))
        | (ModelPattern::Braced(inner1, _), ModelPattern::Braced(inner2, _))
        | (ModelPattern::Parenthesized(inner1, _), ModelPattern::Parenthesized(inner2, _)) => {
            sequence_structure_eq(inner1, inner2)
        }
        (ModelPattern::Optional(inner1, _), ModelPattern::Optional(inner2, _))
        | (ModelPattern::Repeat(inner1, _), ModelPattern::Repeat(inner2, _))
        | (ModelPattern::Plus(inner1, _), ModelPattern::Plus(inner2, _))
        | (ModelPattern::Peek(inner1, _), ModelPattern::Peek(inner2, _))
        | (ModelPattern::Not(inner1, _), ModelPattern::Not(inner2, _)) => {
            pattern_structure_eq(inner1, inner2)
        }
        (
            ModelPattern::Recover {
                body: b1, sync: s1, ..
            },
            ModelPattern::Recover {
                body: b2, sync: s2, ..
            },
        ) => pattern_structure_eq(b1, b2) && pattern_structure_eq(s1, s2),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_resolve_token_types_valid() {
        let kws = HashSet::new();
        let lit: syn::LitStr = parse_quote!("fn");
        let types = resolve_token_types(&lit, &kws).unwrap();
        assert_eq!(types.len(), 1);
    }
}
