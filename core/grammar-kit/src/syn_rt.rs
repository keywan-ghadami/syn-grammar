use super::ParseContext;
use syn::parse::discouraged::Speculative;
use syn::parse::ParseStream;
use syn::Result;

/// Evaluates a parser on a fork.
/// - Returns `Ok(Some)` if successful.
/// - Returns `Ok(None)` if it fails WITHOUT consuming tokens (Shallow Error).
/// - Returns `Err` if it fails AFTER consuming tokens (Deep Error).
pub fn attempt_pure<T>(
    input: ParseStream,
    ctx: &mut ParseContext,
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
            // Consumption Tracking Heuristic:
            // If the error span starts strictly after the current input span,
            // the parser consumed tokens and hit a deep syntax error.
            let err_start = e.span().start();
            let is_deep = err_start.line > input_start.line
                || (err_start.line == input_start.line && err_start.column > input_start.column);

            if is_deep {
                Err(e) // Escalate deep error immediately!
            } else {
                Ok(None) // Shallow error, rollback and return None
            }
        }
    }
}

/// A pure, state-free combinator for separated lists.
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

    // 1. Try to parse the very first item
    match attempt_pure(input, ctx, &mut item_parser) {
        Ok(Some(item)) => items.push(item),
        Ok(None) => {
            if min > 0 {
                let msg = format!("expected at least {} items, found 0", min);
                return Err(syn::Error::new(input.span(), msg));
            }
            return Ok(items); // Valid empty list
        }
        Err(e) => return Err(e), // Deep error escalated
    }

    // 2. Loop for subsequent items
    loop {
        let sep_fork = input.fork();
        match attempt_pure(&sep_fork, ctx, &mut sep_parser) {
            Ok(Some(_)) => {
                // Separator found on fork! Now check for the mandatory item.
                let item_fork = sep_fork.fork();
                match attempt_pure(&item_fork, ctx, &mut item_parser) {
                    Ok(Some(item)) => {
                        // Both sep and item succeeded. Commit everything.
                        input.advance_to(&item_fork);
                        items.push(item);
                    }
                    Ok(None) => {
                        // Separator found, but no item followed (Shallow error on item).
                        if trailing {
                            // Trailing comma allowed. Commit the separator, end the list.
                            input.advance_to(&sep_fork);
                            break;
                        } else {
                            // Trailing comma not allowed.
                            let msg = format!("unexpected end of input, expected {}", item_name.unwrap_or("item"));
                            return Err(syn::Error::new(sep_fork.span(), msg));
                        }
                    }
                    Err(e) => return Err(e), // Deep error in item, escalate immediately!
                }
            }
            Ok(None) => break, // No separator found, natural end of list.
            Err(e) => return Err(e),
        }
    }

    if items.len() < min {
        let msg = format!("expected at least {} items, found {}", min, items.len());
        return Err(syn::Error::new(input.span(), msg));
    }

    Ok(items)
}
