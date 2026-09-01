use super::pattern;
use super::CodegenContext;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::collections::HashMap;
use syn::Result;
use syn_grammar_model::{analysis, model::*};

pub fn generate_rule(rule: &Rule, ctx: &CodegenContext) -> Result<TokenStream> {
    let name = &rule.name;
    let context_name = name.to_string().replace("_", " ");
    let fn_name = format_ident!("parse_{}", name);
    let impl_name = format_ident!("parse_{}_impl", name);
    let ret_type = &rule.return_type;
    let attrs = &rule.attrs;
    let generics = &rule.generics;

    let impl_attrs: Vec<&syn::Attribute> = attrs
        .iter()
        .filter(|a| {
            let p = a.path();
            p.is_ident("cfg")
                || p.is_ident("cfg_attr")
                || p.is_ident("allow")
                || p.is_ident("warn")
                || p.is_ident("deny")
                || p.is_ident("forbid")
        })
        .collect();

    let default_doc = if attrs.iter().any(|a| a.path().is_ident("doc")) {
        quote!()
    } else {
        let msg = format!("Parser for rule `{}`.", name);
        quote!(#[doc = #msg])
    };

    let params: Vec<_> = rule
        .params
        .iter()
        .filter_map(|p| {
            p.ty.as_ref().map(|t| {
                let name = &p.name;
                quote! { , #name : #t }
            })
        })
        .collect();
    let param_names: Vec<_> = rule
        .params
        .iter()
        .filter_map(|p| {
            p.ty.as_ref().map(|_| {
                let name = &p.name;
                quote! { , #name }
            })
        })
        .collect();

    let is_public = rule.is_pub || name == "main";
    let vis = if is_public { quote!(pub) } else { quote!() };
    let (recursive_refs, base_refs) = analysis::split_left_recursive(name, &rule.variants);
    let where_clause = &generics.where_clause;

    // Eigener Kontext fuer diese Regel, damit tiefere Muster ihren Namen kennen.
    let rule_ctx = CodegenContext {
        grammar: ctx.grammar,
        custom_keywords: ctx.custom_keywords,
        current_rule: context_name.clone(),
    };
    let ctx = &rule_ctx;

    let lexical_block_start = if rule.is_lexical {
        quote! { ctx.enter_lexical(); }
    } else {
        quote! {}
    };
    let lexical_block_end = if rule.is_lexical {
        quote! { ctx.exit_mode(); }
    } else {
        quote! {}
    };

    let body = if !recursive_refs.is_empty() {
        if base_refs.is_empty() {
            return Err(syn::Error::new(
                name.span(),
                "Left-recursive rule requires at least one non-recursive base variant.",
            ));
        }

        let base_owned: Vec<RuleVariant> = base_refs.into_iter().cloned().collect();
        let recursive_owned: Vec<RuleVariant> = recursive_refs.into_iter().cloned().collect();
        let base_logic = generate_variants_internal(&base_owned, true, ctx)?;
        let loop_logic = generate_recursive_loop_body(&recursive_owned, ctx)?;

        quote! {
            let mut lhs = {
                let base_parser = |input: &rt::Strom<'a>, ctx: &mut rt::ParseContext<'a>| -> rt::StreamResult<'a, #ret_type> {
                    #base_logic
                };
                base_parser(input, ctx)?
            };
            loop {
                #loop_logic
                break;
            }
            Ok(lhs)
        }
    } else {
        generate_variants_internal(&rule.variants, true, ctx)?
    };

    Ok(quote! {
        #(#attrs)*
        #default_doc
        #vis fn #fn_name(input: syn::parse::ParseStream #(#params)*) -> syn::Result<#ret_type> #where_clause {
            // `ParseStream<'z>` ist `&'z ParseBuffer<'z>` und passt damit direkt
            // auf `&rt::Strom<'a>` mit `'a = 'z`. Der Strom wird von der Regel
            // selbst vorgerueckt - der frueher noetige `input.step`-Umweg (nur
            // dort war die Cursor-Lebensdauer benennbar) entfaellt.
            let mut ctx = rt::ParseContext::new();
            match #impl_name(input, &mut ctx #(#param_names)*) {
                Ok(res) => {
                    // Die Regel ist aufgegangen, hat aber nicht alles verbraucht.
                    // Wurde unterwegs ein Grund gemerkt, warum es nicht weiterging,
                    // ist der die Antwort - sonst meldet syn nur "unexpected token".
                    if !input.is_empty() {
                        if let Some(f) = ctx.furthest.clone() {
                            let mut f = f;
                            f.push_rule(#context_name);
                            return Err(syn::Error::new(f.span, f.to_string()));
                        }
                    }
                    Ok(res)
                }
                Err(e) => {
                    // Der zurueckgegebene Fehler ist nicht zwingend der
                    // aussagekraeftigste - ein weiter gekommener kann unterwegs
                    // von einem erfolgreichen Zuruecksetzen ueberdeckt worden sein.
                    let mut e = ctx.best_error(e);
                    e.push_rule(#context_name);
                    Err(syn::Error::new(e.span, e.to_string()))
                }
            }
        }

        #[doc(hidden)]
        #(#impl_attrs)*
        pub fn #impl_name<'a>(input: &rt::Strom<'a>, ctx: &mut rt::ParseContext<'a> #(#params)*) -> rt::StreamResult<'a, #ret_type> #where_clause {
            // Der Regelname liegt waehrend des Rumpfs auf dem lebenden Stapel, damit
            // ein hier gemerkter (statt herausgereichter) Fehler ihn mitbekommt.
            // enter/exit umschliessen die Rumpf-Closure - alle frueh zurueckkehrenden
            // Pfade darin (Cut, is_fatal, unique-Peek) verlassen nur die Closure, das
            // Paar bleibt also automatisch balanciert.
            ctx.enter_rule(#context_name);
            #lexical_block_start
            let _res = (|| -> rt::StreamResult<'a, #ret_type> {
                #body
            })();
            #lexical_block_end
            ctx.exit_rule();

            match _res {
                Ok(res) => Ok(res),
                Err(mut e) => {
                    e.push_rule(#context_name);
                    Err(e)
                }
            }
        }
    })
}

fn generate_recursive_loop_body(
    variants: &[RuleVariant],
    ctx: &CodegenContext,
) -> Result<TokenStream> {
    let arms = variants.iter().map(|variant| {
        let tail_pattern = &variant.pattern[1..];
        let lhs_binding = match &variant.pattern[0] {
            ModelPattern::RuleCall { binding: Some(b), .. } => Some(b),
            _ => None
        };

        let bind_stmt = if let Some(b) = lhs_binding { quote! { let #b = lhs.clone(); } } else { quote! {} };
        let logic = pattern::generate_sequence(tail_pattern, &variant.action, ctx)?;
        let peek_token_obj = tail_pattern.first().and_then(|f| analysis::get_simple_peek(f, ctx.custom_keywords).ok().flatten());

        let arm_logic = quote! {
            let _start_cursor = input.cursor();
            // Auf einer Gabel versuchen: schlaegt der Arm fehl, bleibt der Strom
            // stehen, wo er war. Beim Cursor-Design war das gratis (den neuen
            // Cursor einfach nicht benutzen), hier kostet es die Gabel.
            let _gabel = rt::gabel(input);
            let _arm_res = (|| -> rt::StreamResult<'a, _> {
                let input = &_gabel;
                #bind_stmt
                #logic
            })();

            match _arm_res {
                Ok(new_lhs) => {
                    let next_cursor = _gabel.cursor();
                    // Cursorvergleich, nicht Positionsvergleich: `Cursor` ist
                    // `PartialEq` (Zeigergleichheit im gemeinsamen TokenBuffer),
                    // und genau das ist die Frage - stand der Parser danach an
                    // derselben Stelle? Ueber `span().start()` waeren bis Rust
                    // 1.87 im Prozedurmakro ALLE Positionen (0,0) gewesen, womit
                    // jede linksrekursive Regel sofort abgebrochen haette.
                    if _start_cursor == next_cursor {
                        return Err(rt::ParseError::at_cursor(_start_cursor, "Left-recursive rule matched empty string").with_priority(50));
                    }
                    rt::uebernehmen(input, &_gabel);
                    lhs = new_lhs;
                    continue;
                }
                Err(e) => {
                    if e.priority >= 50 { return Err(e); }
                    // Die Schleife endet hier regulaer mit dem bisherigen `lhs`.
                    // Ohne dieses Merken ginge der Grund, warum nicht weiter
                    // expandiert wurde, ersatzlos verloren.
                    ctx.record_failure(&e);
                }
            }
        };

        if let Some(token_code) = peek_token_obj {
            Ok(quote! { if rt::peek_syn(input.cursor(), #token_code) { #arm_logic } })
        } else {
            Ok(arm_logic)
        }
    }).collect::<Result<Vec<_>>>()?;
    Ok(quote! { #(#arms)* })
}

pub fn generate_variants_internal(
    variants: &[RuleVariant],
    is_top_level: bool,
    ctx: &CodegenContext,
) -> Result<TokenStream> {
    if variants.is_empty() {
        return Ok(
            quote! { Err(rt::ParseError::at_cursor(input.cursor(), "No variants defined")) },
        );
    }

    let mut token_counts = HashMap::new();
    for v in variants {
        let is_nullable = v.pattern.first().is_none_or(analysis::is_nullable);
        if !is_nullable {
            if let Some(token_str) = analysis::get_peek_token_string(&v.pattern) {
                *token_counts.entry(token_str).or_insert(0) += 1;
            }
        }
    }

    let arms = variants.iter().map(|variant| {
        let label_str = if let Some(l) = &variant.label { Some(l.clone()) } else { analysis::get_peek_token_string(&variant.pattern) };
        let label_lit = if let Some(l) = &label_str { quote!(Some(#l)) } else { quote!(None::<&str>) };

        let cut_info = analysis::find_cut(&variant.pattern);
        let first_pat = variant.pattern.first();
        let is_nullable = first_pat.is_none_or(analysis::is_nullable);
        let peek_token_obj = if !is_nullable { first_pat.and_then(|f| analysis::get_simple_peek(f, ctx.custom_keywords).ok().flatten()) } else { None };
        let peek_str = if !is_nullable { analysis::get_peek_token_string(&variant.pattern) } else { None };
        let is_unique = if let (_, Some(token_key)) = (&peek_token_obj, &peek_str) {
            token_counts.get(token_key).map(|c| *c == 1).unwrap_or(false)
        } else { false };

        let logic = if let Some(cut) = cut_info {
            let pre_logic = pattern::generate_sequence_steps(cut.pre_cut, ctx)?;
            let post_logic = pattern::generate_sequence_steps(cut.post_cut, ctx)?;
            let action = &variant.action;
            let pre_bindings = analysis::collect_bindings(cut.pre_cut);

            let cut_block = quote! {
                let _gabel = rt::gabel(input);
                let pre_res = (|| -> rt::StreamResult<'a, _> {
                    let input = &_gabel;
                    #pre_logic
                    Ok((#(#pre_bindings),*))
                })();

                match pre_res {
                    Ok((#(#pre_bindings),*)) => {
                        // Der Teil vor dem Cut ist aufgegangen. Ab hier laeuft
                        // alles auf derselben Gabel weiter; ein Fehler danach ist
                        // fatal, also wird ohnehin nicht zurueckgesetzt.
                        let post_res = (|| -> rt::StreamResult<'a, _> {
                            let input = &_gabel;
                            #post_logic
                            Ok((|| -> syn::Result<_> { Ok({ #action }) })().map_err(rt::ParseError::from)?)
                        })();
                        match post_res {
                            Ok(res) => {
                                rt::uebernehmen(input, &_gabel);
                                return Ok(res);
                            }
                            // CUT: die Ableitung ist festgelegt, Zuruecksetzen sinnlos.
                            Err(e) => return Err(e.as_fatal()),
                        }
                    }
                    Err(e) => {
                        // Nur ein Cut schliesst kurz. Ein `fail(..)` ist hochprior,
                        // aber nicht fatal - es muss in den Vergleich, sonst gewinnt
                        // es auch gegen einen weiter gekommenen Fehler.
                        if e.is_fatal { return Err(e); }
                        // Ohne Fortschritt ist die Alternative an ihrer Grenze
                        // gescheitert. Dann zaehlt ihr Label als Erwartung, statt
                        // die interne Meldung nach aussen zu tragen (ADR 13, Punkt 6).
                        if e.at == Some(_start_cursor) {
                            if let Some(lbl) = #label_lit { _expected.push(lbl.to_string()); }
                        }
                        // Nur MIT Fortschritt in den Hochwasserstand: gewinnt eine
                        // spaetere Alternative, wird `_best_err` verworfen - und mit
                        // ihm der aussagekraeftigste Grund, falls hinterher noch
                        // Eingabe uebrig bleibt. Ein Fehler ohne Fortschritt ist
                        // dagegen keine "weiteste Fehlschlagstelle"; er gehoert in die
                        // Erwartungsliste oben, nicht in den globalen Mark - sonst
                        // verdraengt die Aggregation eines optionalen Teilmusters das
                        // Label des Elements (ADR 13, Punkt 6).
                        else {
                            ctx.record_failure(&e);
                        }
                        _best_err = Some(_best_err.map_or(e.clone(), |b| b.merge(e)));
                    }
                }
            };
            cut_block
        } else {
            let inner_logic = pattern::generate_sequence(&variant.pattern, &variant.action, ctx)?;
            quote! {
                let _gabel = rt::gabel(input);
                let _arm_res = (|| -> rt::StreamResult<'a, _> {
                    let input = &_gabel;
                    #inner_logic
                })();
                match _arm_res {
                    Ok(res) => {
                        rt::uebernehmen(input, &_gabel);
                        return Ok(res);
                    }
                    Err(e) => {
                        // Nur ein Cut schliesst kurz. Ein `fail(..)` ist hochprior,
                        // aber nicht fatal - es muss in den Vergleich, sonst gewinnt
                        // es auch gegen einen weiter gekommenen Fehler.
                        if e.is_fatal { return Err(e); }
                        // Ohne Fortschritt ist die Alternative an ihrer Grenze
                        // gescheitert. Dann zaehlt ihr Label als Erwartung, statt
                        // die interne Meldung nach aussen zu tragen (ADR 13, Punkt 6).
                        if e.at == Some(_start_cursor) {
                            if let Some(lbl) = #label_lit { _expected.push(lbl.to_string()); }
                        }
                        // Nur MIT Fortschritt in den Hochwasserstand: gewinnt eine
                        // spaetere Alternative, wird `_best_err` verworfen - und mit
                        // ihm der aussagekraeftigste Grund, falls hinterher noch
                        // Eingabe uebrig bleibt. Ein Fehler ohne Fortschritt ist
                        // dagegen keine "weiteste Fehlschlagstelle"; er gehoert in die
                        // Erwartungsliste oben, nicht in den globalen Mark - sonst
                        // verdraengt die Aggregation eines optionalen Teilmusters das
                        // Label des Elements (ADR 13, Punkt 6).
                        else {
                            ctx.record_failure(&e);
                        }
                        _best_err = Some(_best_err.map_or(e.clone(), |b| b.merge(e)));
                    }
                }
            }
        };

        // Scheitert der Peek, laeuft der Zweig gar nicht erst und erzeugt keinen
        // Fehler. Sein Label ist dann trotzdem eine gueltige Erwartung an dieser
        // Stelle und muss in die Aufzaehlung (ADR 13, Punkt 6) - sonst bleibt nur
        // die nichtssagende Meldung "No matching rule variant found".
        let sonst_erwartet = quote! {
            else if let Some(lbl) = #label_lit { _expected.push(lbl.to_string()); }
        };

        if is_unique {
            let token_code = peek_token_obj.as_ref().unwrap();
            Ok(quote! {
                if rt::peek_syn(_start_cursor, #token_code) {
                    #logic
                    // Das Peek-Token gehoert eindeutig zu dieser Variante: scheitert
                    // sie, brauchen die uebrigen gar nicht mehr versucht zu werden.
                    // Das leistet bereits das `return` - die Prioritaet wird NICHT
                    // angehoben. Ein struktureller Fehler waere hier fatal im Sinne
                    // von "nicht behebbar", und genau das machte recover() auf jeder
                    // Regel mit eindeutigem Anfangstoken unbrauchbar.
                    if let Some(err) = _best_err.take() {
                        return Err(err);
                    } else {
                        return Err(rt::ParseError::at_cursor(_start_cursor, "propagating unique variant error"));
                    }
                }
                #sonst_erwartet
            })
        } else if let Some(token_code) = peek_token_obj {
            Ok(quote! {
                if rt::peek_syn(_start_cursor, #token_code) { #logic }
                #sonst_erwartet
            })
        } else {
            Ok(logic)
        }
    }).collect::<Result<Vec<_>>>()?;

    let error_msg = if is_top_level {
        "No matching rule variant found"
    } else {
        "No matching variant in group"
    };

    Ok(quote! {
        let mut _best_err: Option<rt::ParseError> = None;
        let mut _expected: Vec<String> = Vec::new();
        let _start_cursor = input.cursor();

        #(#arms)*

        Err(rt::finish_variants(_best_err, _expected, _start_cursor, #error_msg))
    })
}
