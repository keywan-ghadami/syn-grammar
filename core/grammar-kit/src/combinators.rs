// combinators.rs

pub fn attempt_labeled<T, F>(
    input: ParseStream,
    ctx: &mut ParseContext,
    label: Option<&str>,
    parser: F,
) -> Result<Option<T>>
where
    F: FnOnce(ParseStream, &mut ParseContext) -> Result<T>,
{
    let fork = input.fork();
    
    // 1. Transaktion starten (nimmt automatisch Snapshots)
    let transaction = ParseTransaction::begin(ctx, input.span());

    // 2. Parsen
    match parser(&fork, ctx) {
        Ok(val) => {
            // 3a. Erfolg: Input vorschieben und Zustand committen
            input.advance_to(&fork);
            transaction.commit();
            Ok(Some(val))
        }
        Err(e) => {
            // 3b. Fehler: Automatischer Rollback und Fehler-Merging
            let bubbled_err = transaction.rollback(e, label);
            
            if bubbled_err.to_string().contains("__BUBBLE__") || ctx.check_fatal() {
                return Err(bubbled_err);
            }
            
            Ok(None)
        }
    }
}
