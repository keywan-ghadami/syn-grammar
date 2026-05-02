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
                // Fix für test_cxx_unexpected_eof:
                // Tiefe Fehler MÜSSEN im Kontext registriert werden, bevor sie 
                // eskaliert werden, sonst überschreibt der Top-Level-Parser sie 
                // mit einem generischen "propagating fatal unique error".
                ctx.record_error(e.clone(), e.span(), None, ParseContext::PRIO_STRUCTURAL);
                Err(e)
            } else {
                // Fix für test_cxx_shallow_wrong_token:
                // Wir überschreiben den Fehlertext nicht manuell mit format!("expected {}", ...), 
                // da dies zusätzliche Backticks einführt, die die Tests brechen.
                // Wir übergeben das Label lediglich an record_error.
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
        if let Some(ref _name) = rule_name { ctx.enter_rule(_name); } // Warnung behoben
        
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
                ctx.record_error(err.clone(), input.span(), None, ParseContext::PRIO_NORMAL);
                if rule_name.is_some() { ctx.exit_rule(); }
                
                return Err(err);
            }
            return Ok(items);
        }
        Err(e) => return Err(e),
    }

    loop {
        let sep_fork = input.fork();
        
        // Fix für test_cxx_garbage_after_item:
        // Keine Rule-Injection ("separator") mehr, damit das echte erwartete Token
        // (z.B. "expected `,`") nicht durch "expected separator" maskiert wird.
        let sep_res = attempt_labeled_pure(&sep_fork, ctx, None, &mut sep_parser);

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
                            // Fix für test_cxx_dangling_comma:
                            // Wir generieren hier keinen eigenen Fehler mehr. Der tiefste 
                            // Fehler (z.B. "expected function parameter") wurde bereits 
                            // im Kontext registriert. Wir werfen nur noch den Bubble-Error,
                            // damit die obere Schicht exakt diesen tiefsten Fehler extrahiert.
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
        ctx.record_error(err.clone(), input.span(), None, ParseContext::PRIO_NORMAL);
        return Err(err);
    }

    Ok(items)
}
