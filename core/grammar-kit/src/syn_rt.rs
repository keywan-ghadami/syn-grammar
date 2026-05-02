use crate::ParseContext;
use syn::parse::discouraged::Speculative;
use syn::parse::ParseStream;
use syn::Result;

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

            if is_deep {
                // FIX for test_cxx_unexpected_eof:
                // Do NOT record `e` here. `e` is often the dummy bubble error 
                // ("propagating fatal unique error") from the inner macro logic.
                // The actual deep semantic error is already safely stored in the context.
                Err(e)
            } else {
                // FIX for test_cxx_garbage_after_item:
                // Pass the label to `record_error` so it wins tie-breakers (less specific context),
                // but do NOT overwrite the `e` message itself (keeps "expected `,`").
                ctx.record_error(
                    e.clone(),
                    e.span(),
                    label.map(|s| s.to_string()),
                    ParseContext::PRIO_NORMAL,
                );
                Ok(None)
            }
        }
    }
}

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
        if let Some(ref _name) = rule_name { ctx.enter_rule(_name); }
        
        let res = attempt_labeled_pure(input, ctx, item_name, &mut item_parser);
        
        if rule_name.is_some() { ctx.exit_rule(); }
        res
    };

    match parse_item(input, ctx, 1) {
        Ok(Some(item)) => items.push(item),
        Ok(None) => {
            if min > 0 {
                let msg = item_name
                    .map(|n| format!("expected {}", n))
                    .unwrap_or_else(|| format!("expected at least {} items", min));
                let err = syn::Error::new(input.span(), msg);
                
                let rule_name = item_name.map(|n| format!("{} 1", n));
                if let Some(ref _name) = rule_name { ctx.enter_rule(_name); }
                
                // FIX for test_cxx_shallow_wrong_token:
                // Use PRIO_STRUCTURAL (50) to cleanly override any internal 
                // PRIO_AGGREGATED (20) errors from the failed item rule.
                ctx.record_error(err.clone(), input.span(), None, ParseContext::PRIO_STRUCTURAL);
                
                if rule_name.is_some() { ctx.exit_rule(); }
                
                return Err(syn::Error::new(input.span(), "__BUBBLE__"));
            }
            return Ok(items);
        }
        Err(e) => return Err(e),
    }

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
                                .unwrap_or_else(|| "unexpected end of input, expected item".to_string());
                            let err = syn::Error::new(item_fork.span(), msg);
                            
                            let rule_name = item_name.map(|n| format!("{} {}", n, next_idx));
                            if let Some(ref _name) = rule_name { ctx.enter_rule(_name); }
                            
                            // FIX for test_cxx_dangling_comma:
                            // Use PRIO_STRUCTURAL to override internal shallow rule errors
                            ctx.record_error(err.clone(), item_fork.span(), None, ParseContext::PRIO_STRUCTURAL);
                            
                            if rule_name.is_some() { ctx.exit_rule(); }
                            
                            return Err(syn::Error::new(item_fork.span(), "__BUBBLE__"));
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
        ctx.record_error(err.clone(), input.span(), None, ParseContext::PRIO_STRUCTURAL);
        return Err(syn::Error::new(input.span(), "__BUBBLE__"));
    }

    Ok(items)
}
