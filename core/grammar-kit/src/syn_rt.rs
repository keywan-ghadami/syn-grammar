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
                Err(e) 
            } else {
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
    let actual_item_name = item_name.unwrap_or("item");

    let mut parse_item = |input: ParseStream, ctx: &mut ParseContext, idx: usize| -> Result<Option<T>> {
        let rule_name = format!("{} {}", actual_item_name, idx);
        ctx.enter_rule(&rule_name);
        
        let res = attempt_labeled_pure(input, ctx, item_name, &mut item_parser);
        
        ctx.exit_rule();
        res
    };

    match parse_item(input, ctx, 1) {
        Ok(Some(item)) => items.push(item),
        Ok(None) => {
            if min > 0 {
                let msg = if input.is_empty() {
                    if ctx.is_in_group() {
                        format!("unexpected end of group, expected {}", actual_item_name)
                    } else {
                        format!("unexpected end of input, expected {}", actual_item_name)
                    }
                } else {
                    format!("expected {}", actual_item_name)
                };
                let err = syn::Error::new(input.span(), msg);
                
                let rule_name = format!("{} 1", actual_item_name);
                ctx.enter_rule(&rule_name);
                ctx.record_error(err.clone(), input.span(), None, ParseContext::PRIO_STRUCTURAL);
                ctx.exit_rule();
                
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
                            let msg = if item_fork.is_empty() {
                                if ctx.is_in_group() {
                                    format!("unexpected end of group, expected {}", actual_item_name)
                                } else {
                                    format!("unexpected end of input, expected {}", actual_item_name)
                                }
                            } else {
                                format!("expected {}", actual_item_name)
                            };
                            let err = syn::Error::new(item_fork.span(), msg);
                            
                            let rule_name = format!("{} {}", actual_item_name, next_idx);
                            ctx.enter_rule(&rule_name);
                            ctx.record_error(err, item_fork.span(), None, ParseContext::PRIO_STRUCTURAL);
                            ctx.exit_rule();
                            
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
        let msg = if input.is_empty() {
            if ctx.is_in_group() {
                format!("unexpected end of group, expected at least {} items, found {}", min, items.len())
            } else {
                format!("unexpected end of input, expected at least {} items, found {}", min, items.len())
            }
        } else {
            format!("expected at least {} items, found {}", min, items.len())
        };
        let err = syn::Error::new(input.span(), msg);
        ctx.record_error(err, input.span(), None, ParseContext::PRIO_STRUCTURAL);
        return Err(syn::Error::new(input.span(), "__BUBBLE__"));
    }

    Ok(items)
}