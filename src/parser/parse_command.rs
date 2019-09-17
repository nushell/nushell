use crate::errors::{ArgumentError, ShellError};
use crate::parser::hir::syntax_shape::{expand_expr, spaced};
use crate::parser::registry::{NamedType, PositionalType, Signature};
use crate::parser::TokensIterator;
use crate::parser::{
    hir::{self, ExpandContext, NamedArguments},
    Flag,
};
use crate::traits::ToDebug;
use crate::{Tag, Tagged, Text};
use log::trace;

pub fn parse_command_tail(
    config: &Signature,
    context: &ExpandContext,
    tail: &mut TokensIterator,
    command_tag: Tag,
) -> Result<Option<(Option<Vec<hir::Expression>>, Option<NamedArguments>)>, ShellError> {
    let mut named = NamedArguments::new();
    trace_remaining("nodes", tail.clone(), context.source());

    for (name, kind) in &config.named {
        trace!(target: "nu::parse", "looking for {} : {:?}", name, kind);

        match kind {
            NamedType::Switch => {
                let flag = extract_switch(name, tail, context.source());

                named.insert_switch(name, flag);
            }
            NamedType::Mandatory(syntax_type) => {
                match extract_mandatory(config, name, tail, context.source(), command_tag) {
                    Err(err) => return Err(err), // produce a correct diagnostic
                    Ok((pos, flag)) => {
                        tail.move_to(pos);

                        if tail.at_end() {
                            return Err(ShellError::argument_error(
                                config.name.clone(),
                                ArgumentError::MissingValueForName(name.to_string()),
                                flag.tag(),
                            ));
                        }

                        let expr = expand_expr(&spaced(*syntax_type), tail, context)?;

                        tail.restart();
                        named.insert_mandatory(name, expr);
                    }
                }
            }
            NamedType::Optional(syntax_type) => {
                match extract_optional(name, tail, context.source()) {
                    Err(err) => return Err(err), // produce a correct diagnostic
                    Ok(Some((pos, flag))) => {
                        tail.move_to(pos);

                        if tail.at_end() {
                            return Err(ShellError::argument_error(
                                config.name.clone(),
                                ArgumentError::MissingValueForName(name.to_string()),
                                flag.tag(),
                            ));
                        }

                        let expr = expand_expr(&spaced(*syntax_type), tail, context);

                        match expr {
                            Err(_) => named.insert_optional(name, None),
                            Ok(expr) => named.insert_optional(name, Some(expr)),
                        }

                        tail.restart();
                    }

                    Ok(None) => {
                        tail.restart();
                        named.insert_optional(name, None);
                    }
                }
            }
        };
    }

    trace_remaining("after named", tail.clone(), context.source());

    let mut positional = vec![];

    for arg in &config.positional {
        trace!("Processing positional {:?}", arg);

        match arg {
            PositionalType::Mandatory(..) => {
                if tail.at_end() {
                    return Err(ShellError::argument_error(
                        config.name.clone(),
                        ArgumentError::MissingMandatoryPositional(arg.name().to_string()),
                        command_tag,
                    ));
                }
            }

            PositionalType::Optional(..) => {
                if tail.at_end() {
                    break;
                }
            }
        }

        let result = expand_expr(&spaced(arg.syntax_type()), tail, context)?;

        positional.push(result);
    }

    trace_remaining("after positional", tail.clone(), context.source());

    if let Some(syntax_type) = config.rest_positional {
        let mut out = vec![];

        loop {
            if tail.at_end_possible_ws() {
                break;
            }

            let next = expand_expr(&spaced(syntax_type), tail, context)?;

            out.push(next);
        }

        positional.extend(out);
    }

    trace_remaining("after rest", tail.clone(), context.source());

    trace!("Constructed positional={:?} named={:?}", positional, named);

    let positional = if positional.len() == 0 {
        None
    } else {
        Some(positional)
    };

    // TODO: Error if extra unconsumed positional arguments

    let named = if named.named.is_empty() {
        None
    } else {
        Some(named)
    };

    trace!("Normalized positional={:?} named={:?}", positional, named);

    Ok(Some((positional, named)))
}

fn extract_switch(name: &str, tokens: &mut hir::TokensIterator<'_>, source: &Text) -> Option<Flag> {
    tokens
        .extract(|t| t.as_flag(name, source))
        .map(|(_pos, flag)| flag.item)
}

fn extract_mandatory(
    config: &Signature,
    name: &str,
    tokens: &mut hir::TokensIterator<'_>,
    source: &Text,
    tag: Tag,
) -> Result<(usize, Tagged<Flag>), ShellError> {
    let flag = tokens.extract(|t| t.as_flag(name, source));

    match flag {
        None => Err(ShellError::argument_error(
            config.name.clone(),
            ArgumentError::MissingMandatoryFlag(name.to_string()),
            tag,
        )),

        Some((pos, flag)) => {
            tokens.remove(pos);
            Ok((pos, flag))
        }
    }
}

fn extract_optional(
    name: &str,
    tokens: &mut hir::TokensIterator<'_>,
    source: &Text,
) -> Result<(Option<(usize, Tagged<Flag>)>), ShellError> {
    let flag = tokens.extract(|t| t.as_flag(name, source));

    match flag {
        None => Ok(None),
        Some((pos, flag)) => {
            tokens.remove(pos);
            Ok(Some((pos, flag)))
        }
    }
}

pub fn trace_remaining(desc: &'static str, tail: hir::TokensIterator<'_>, source: &Text) {
    trace!(
        "{} = {:?}",
        desc,
        itertools::join(
            tail.debug_remaining()
                .iter()
                .map(|i| format!("%{}%", i.debug(&source))),
            " "
        )
    );
}
