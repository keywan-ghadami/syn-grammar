use syn::parse::discouraged::Speculative;
use syn::parse::ParseStream;
use syn::Result;
use crate::{ParseContext, transaction::ParseTransaction};

#[inline]
pub fn attempt<T, F>(input: ParseStream, ctx: &mut ParseContext, parser: F) -> Result<Option<T>>
where F: FnOnce(ParseStream, &mut ParseContext) -> Result<T>,
{
    attempt_labeled(input, ctx, None, parser)
}

#[inline]
pub fn attempt_labeled<T, F>(
    input: ParseStream,
    ctx: &mut ParseContext,
    label: Option<&str>,
    parser: F,
) -> Result<Option<T>>
where F: FnOnce(ParseStream, &mut ParseContext) -> Result<T>,
{
    let fork = input.fork();
    let transaction = ParseTransaction::begin(ctx, input.span());

    match parser(&fork, transaction.ctx) {
        Ok(val) => {
            input.advance_to(&fork);
            transaction.commit();
            Ok(Some(val))
        }
        Err(e) => {
            let bubbled_err = transaction.rollback(e, label);
            if bubbled_err.to_string().contains("__BUBBLE__") || bubbled_err.to_string().contains("__DUMMY_ERR_BUBBLE__") || ctx.check_fatal() {
                return Err(bubbled_err);
            }
            Ok(None)
        }
    }
}

#[inline]
pub fn peek<T, F>(input: ParseStream, ctx: &mut ParseContext, parser: F) -> Result<T>
where F: FnOnce(ParseStream, &mut ParseContext) -> Result<T>,
{
    let fork = input.fork();
    let transaction = ParseTransaction::begin(ctx, input.span());
    let res = parser(&fork, transaction.ctx);
    // Peek ist destruktiv, wir werfen den neuen Zustand immer weg
    let _ = transaction.rollback(syn::Error::new(input.span(), "peek"), None);
    res
}

#[inline]
pub fn not_check<T, F>(input: ParseStream, ctx: &mut ParseContext, parser: F) -> Result<()>
where F: FnOnce(ParseStream, &mut ParseContext) -> Result<T>,
{
    let fork = input.fork();
    let transaction = ParseTransaction::begin(ctx, input.span());
    let res = parser(&fork, transaction.ctx);
    let _ = transaction.rollback(syn::Error::new(input.span(), "not"), None);

    match res {
        Ok(_) => Err(syn::Error::new(input.span(), "unexpected match")),
        Err(_) => Ok(()),
    }
}

#[inline]
pub fn attempt_recover<T, F>(input: ParseStream, ctx: &mut ParseContext, parser: F) -> Result<Option<T>>
where F: FnOnce(ParseStream, &mut ParseContext) -> Result<T>,
{
    let fork = input.fork();
    let transaction = ParseTransaction::begin(ctx, input.span());

    match parser(&fork, transaction.ctx) {
        Ok(val) => {
            input.advance_to(&fork);
            transaction.commit();
            Ok(Some(val))
        }
        Err(e) => {
            let bubbled_err = transaction.rollback_for_recovery(e);
            if bubbled_err.to_string().contains("__BUBBLE__") || bubbled_err.to_string().contains("__DUMMY_ERR_BUBBLE__") {
                return Err(bubbled_err);
            }
            Ok(None)
        }
    }
}

pub fn parse_separated<T, P, S>(
    input: ParseStream,
    ctx: &mut ParseContext,
    mut item_parser: P,
    mut sep_parser: S,
    min: usize,
    trailing: bool,
    item_name: &str,
) -> Result<Vec<T>>
where
    P: FnMut(ParseStream, &mut ParseContext) -> Result<T>,
    S: FnMut(ParseStream, &mut ParseContext) -> Result<()> ,
{
    let mut items = Vec::new();
    
    let first_item_span = input.span();
    ctx.enter_rule(&format!("{} 1", item_name));
    let first_item = match attempt_labeled(input, ctx, Some(item_name), |i, c| item_parser(i, c)) {
        Ok(Some(item)) => { ctx.exit_rule(); item }
        Ok(None) => {
            ctx.exit_rule();
            if ctx.stop_aggregation(first_item_span) { return Err(syn::Error::new(first_item_span, "__BUBBLE__")); }
            return Ok(items);
        }
        Err(e) => { ctx.exit_rule(); return Err(e); }
    };
    items.push(first_item);

    loop {
        let pre_sep_span = input.span();
        let sep_fork = input.fork();
        ctx.enter_rule("separator");
        let sep_res = attempt(&sep_fork, ctx, |i, c| sep_parser(i, c));
        ctx.exit_rule();
        
        match sep_res {
            Ok(Some(_)) => {
                let item_fork = sep_fork.fork();
                let next_idx = items.len() + 1;
                let rule_name = format!("{} {}", item_name, next_idx);
                ctx.enter_rule(&rule_name);
                let item_res = attempt_labeled(&item_fork, ctx, Some(item_name), |i, c| item_parser(i, c));
                ctx.exit_rule();
                
                match item_res {
                    Ok(Some(item)) => {
                        input.advance_to(&item_fork);
                        items.push(item);
                    }
                    Ok(None) => {
                        if trailing {
                            input.advance_to(&sep_fork);
                            break;
                        } else {
                            let msg = format!("expected {}", item_name);
                            ctx.record_error(syn::Error::new(item_fork.span(), &msg), item_fork.span(), None, ParseContext::PRIO_STRUCTURAL);
                            break; 
                        }
                    }
                    Err(e) => return Err(e),
                }
            }
            Ok(None) => {
                if ctx.stop_aggregation(pre_sep_span) {
                    return Err(syn::Error::new(pre_sep_span, "__BUBBLE__"));
                }
                break;
            }
            Err(e) => return Err(e), 
        }
    }

    if items.len() < min {
        return ctx.raise_failure(
            &format!("expected at least {} {}s, found {}", min, item_name, items.len()),
            input.span(),
        );
    }

    Ok(items)
}

pub fn parse_repeated<T, P>(
    input: ParseStream,
    ctx: &mut ParseContext,
    mut item_parser: P,
    min: usize,
    item_name: &str,
) -> Result<Vec<T>>
where
    P: FnMut(ParseStream, &mut ParseContext) -> Result<T>,
{
    let mut items = Vec::new();
    loop {
        let loop_start_span = input.span();
        let next_idx = items.len() + 1;
        let rule_name = format!("{} {}", item_name, next_idx);

        ctx.enter_rule(&rule_name);
        let item = match attempt_labeled(input, ctx, Some(item_name), |i, c| item_parser(i, c)) {
            Ok(Some(item)) => item,
            Ok(None) => {
                ctx.exit_rule();
                if ctx.stop_aggregation(loop_start_span) {
                    return Err(syn::Error::new(loop_start_span, "__BUBBLE__"));
                }
                break;
            }
            Err(e) => {
                ctx.exit_rule();
                return Err(e);
            }
        };
        ctx.exit_rule();
        items.push(item);
    }

    if items.len() < min {
        return ctx.raise_failure(
            &format!("expected at least {} {}s, found {}", min, item_name, items.len()),
            input.span()
        );
    }

    Ok(items)
}

pub fn parse_delimited<T, F>(
    input: ParseStream,
    ctx: &mut ParseContext,
    parser: F,
    delimiter: char,
) -> Result<T>
where
    F: FnOnce(ParseStream, &mut ParseContext) -> Result<T>,
{
    let content;
    let final_span: proc_macro2::Span;
    match delimiter {
        '(' => {
            let paren_token = syn::parenthesized!(content in input);
            final_span = paren_token.span.join();
        }
        '{' => {
            let brace_token = syn::braced!(content in input);
            final_span = brace_token.span.join();
        }
        '[' => {
            let bracket_token = syn::bracketed!(content in input);
            final_span = bracket_token.span.join();
        }
        _ => {
            return Err(syn::Error::new(
                input.span(),
                "unsupported delimiter for custom parsing",
            ));
        }
    }
    ctx.record_span(final_span)?;

    ctx.enter_group();
    let res = parser(&content, ctx);
    ctx.exit_group();

    match res {
        Ok(val) => {
            if !content.is_empty() {
                if ctx.stop_aggregation(content.span()) {
                    return Err(syn::Error::new(content.span(), "__BUBBLE__"));
                }
                let err = content.error("unexpected token in delimited group");
                ctx.record_error(err, content.span(), None, ParseContext::PRIO_NORMAL);
                return Err(syn::Error::new(content.span(), "__BUBBLE__"));
            } else {
                Ok(val)
            }
        }
        Err(e) => Err(e)
    }
}
