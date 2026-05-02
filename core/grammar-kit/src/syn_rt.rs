use crate::rt::ParseContext;
use syn::parse::ParseStream;
use syn::Result;

/// Evaluates a parser on a fork, tracking rule context and label priorities.
pub fn attempt_labeled_pure<T>(
    input: ParseStream,
    ctx: &mut ParseContext,
    label: Option<&str>,
    parser: impl FnOnce(ParseStream, &mut ParseContext) -> Result<T>,
) -> Result<Option<T>> {
    let fork = input.fork();
    let input_start = input.span().start();

    match parser(&fork, ctx) {
        Ok(res) => {
            input.advance_to(&fork);
            Ok(Some(res))
        }
        Err(e) => {
            let err_start = e.span().start();
            let is_deep = err_start.line > input_start.line
                || (err_start.line == input_start.line && err_start.column > input_start.column);

            // If it's a shallow error and we have a label, override the error message 
            // for the tie-breaker to prevent leaking internal syntax requirements.
            let err_to_record = if !is_deep && label.is_some() {
                syn::Error::new(e.span(), format!("expected {}", label.unwrap()))
            } else {
                e.clone()
            };

            // Record the failure so the top-level "furthest failure" heuristic
            // can use it if the parent rule ultimately fails.
            ctx.record_error(
                err_to_record,
                e.span(),
                label.map(|s| s.to_string()),
                crate::rt::ParseContext::PRIO_NORMAL,
            );

            if is_deep {
                Err(e) // Escalate deep error immediately!
            } else {
                Ok(None) // Shallow error, rollback and return None
            }
        }
    }
}

/// A pure, state-free combinator for separated lists with full context-stack tracking.
pub fn parse_separated_pure<T, S>(
    input: ParseStream,
    ctx: &mut ParseContext,
    mut item_parser: impl FnMut(ParseStream, &mut ParseContext) -> Result<T>,
    mut sep_parser: impl FnMut(ParseStream, &mut ParseContext) -> Result<S>,
    min: usize,
    trailing: bool,
    item_name: Option<&str>,
) -> Result<Vec<T>> {
    let mut items = Vec::new();

    let mut parse_item = |input: ParseStream, ctx: &mut ParseContext, idx: usize| -> Result<Option<T>> {
        let rule_name = item_name.map(|n| format!("{} {}", n, idx));
        if let Some(ref name) = rule_name { ctx.enter_rule(name); }
        
        let res = attempt_labeled_pure(input, ctx, item_name, &mut item_parser);
        
        if let Some(ref name) = rule_name { ctx.exit_rule(); }
        res
    };

    // 1. Try to parse the very first item
    match parse_item(input, ctx, 1) {
        Ok(Some(item)) => items.push(item),
        Ok(None) => {
            if min > 0 {
                let msg = item_name
                    .map(|n| format!("expected {}", n))
                    .unwrap_or_else(|| format!("expected at least {} items", min));
                let err = syn::Error::new(input.span(), msg);
                
                let rule_name = item_name.map(|n| format!("{} 1", n));
                if let Some(ref name) = rule_name { ctx.enter_rule(name); }
                ctx.record_error(err.clone(), input.span(), None, crate::rt::ParseContext::PRIO_NORMAL);
                if let Some(ref name) = rule_name { ctx.exit_rule(); }
                
                return Err(err);
            }
            return Ok(items);
        }
        Err(e) => return Err(e),
    }

    // 2. Loop for subsequent items
    loop {
        let sep_fork = input.fork();
        
        ctx.enter_rule("separator");
        let sep_res = attempt_labeled_pure(&sep_fork, ctx, Some("separator"), &mut sep_parser);
        ctx.exit_rule();

        match sep_res {
            Ok(Some(_)) => {
                let item_fork = sep_fork.fork();
                let next_idx = items.len() + 1;
                
                match parse_item(&item_fork, ctx, next_idx) {
                    Ok(Some(item)) => {
                        input.advance_to(&item_fork);
                        items.push(item);
                    }
                    Ok(None) => {
                        if trailing {
                            input.advance_to(&sep_fork);
                            break;
                        } else {
                            let msg = item_name
                                .map(|n| format!("unexpected end of input, expected {}", n))
                                .unwrap_or_else(|| "unexpected end of input".to_string());
                            let err = syn::Error::new(item_fork.span(), msg);
                            
                            let rule_name = item_name.map(|n| format!("{} {}", n, next_idx));
                            if let Some(ref name) = rule_name { ctx.enter_rule(name); }
                            ctx.record_error(err.clone(), item_fork.span(), None, crate::rt::ParseContext::PRIO_NORMAL);
                            if let Some(ref name) = rule_name { ctx.exit_rule(); }
                            
                            return Err(err);
                        }
                    }
                    Err(e) => return Err(e),
                }
            }
            Ok(None) => break,
            Err(e) => return Err(e),
        }
    }

    if items.len() < min {
        let msg = format!("expected at least {} items, found {}", min, items.len());
        let err = syn::Error::new(input.span(), msg);
        ctx.record_error(err.clone(), input.span(), None, crate::rt::ParseContext::PRIO_NORMAL);
        return Err(err);
    }

    Ok(items)
}