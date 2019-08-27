use crate::context::Context;
use crate::errors::{ArgumentError, ShellError};
use crate::parser::registry::{NamedType, PositionalType, Signature};
use crate::parser::{baseline_parse_tokens, CallNode};
use crate::parser::{
    hir::{self, NamedArguments},
    Flag, RawToken, TokenNode,
};
use crate::{Span, Tag, Tagged, Text};
use log::trace;

pub fn parse_command(
    config: &Signature,
    context: &Context,
    call: &Tagged<CallNode>,
    source: &Text,
) -> Result<hir::Call, ShellError> {
    let Tagged { item: raw_call, .. } = call;

    trace!("Processing {:?}", config);

    let head = parse_command_head(call.head())?;

    let children: Option<Vec<TokenNode>> = raw_call.children().as_ref().map(|nodes| {
        nodes
            .iter()
            .cloned()
            .filter(|node| match node {
                TokenNode::Whitespace(_) => false,
                _ => true,
            })
            .collect()
    });

    match parse_command_tail(&config, context, children, source, call.span())? {
        None => Ok(hir::Call::new(Box::new(head), None, None)),
        Some((positional, named)) => Ok(hir::Call::new(Box::new(head), positional, named)),
    }
}

fn parse_command_head(head: &TokenNode) -> Result<hir::Expression, ShellError> {
    match head {
        TokenNode::Token(
            spanned @ Tagged {
                item: RawToken::Bare,
                ..
            },
        ) => Ok(spanned.map(|_| hir::RawExpression::Literal(hir::Literal::Bare))),

        TokenNode::Token(Tagged {
            item: RawToken::String(inner_span),
            tag: Tag { span, origin: None },
        }) => Ok(Tagged::from_simple_spanned_item(
            hir::RawExpression::Literal(hir::Literal::String(*inner_span)),
            *span,
        )),

        other => Err(ShellError::unexpected(&format!(
            "command head -> {:?}",
            other
        ))),
    }
}

fn parse_command_tail(
    config: &Signature,
    context: &Context,
    tail: Option<Vec<TokenNode>>,
    source: &Text,
    command_span: Span,
) -> Result<Option<(Option<Vec<hir::Expression>>, Option<NamedArguments>)>, ShellError> {
    let tail = &mut match &tail {
        None => hir::TokensIterator::new(&[]),
        Some(tail) => hir::TokensIterator::new(tail),
    };

    let mut named = NamedArguments::new();

    trace_remaining("nodes", tail.clone(), source);

    for (name, kind) in &config.named {
        trace!(target: "nu::parse", "looking for {} : {:?}", name, kind);

        match kind {
            NamedType::Switch => {
                let flag = extract_switch(name, tail, source);

                named.insert_switch(name, flag);
            }
            NamedType::Mandatory(syntax_type) => {
                match extract_mandatory(config, name, tail, source, command_span) {
                    Err(err) => return Err(err), // produce a correct diagnostic
                    Ok((pos, flag)) => {
                        tail.move_to(pos);

                        if tail.at_end() {
                            return Err(ShellError::argument_error(
                                config.name.clone(),
                                ArgumentError::MissingValueForName(name.to_string()),
                                flag.span(),
                            ));
                        }

                        let expr =
                            hir::baseline_parse_next_expr(tail, context, source, *syntax_type)?;

                        tail.restart();
                        named.insert_mandatory(name, expr);
                    }
                }
            }
            NamedType::Optional(syntax_type) => match extract_optional(name, tail, source) {
                Err(err) => return Err(err), // produce a correct diagnostic
                Ok(Some((pos, flag))) => {
                    tail.move_to(pos);

                    if tail.at_end() {
                        return Err(ShellError::argument_error(
                            config.name.clone(),
                            ArgumentError::MissingValueForName(name.to_string()),
                            flag.span(),
                        ));
                    }

                    let expr = hir::baseline_parse_next_expr(tail, context, source, *syntax_type)?;

                    tail.restart();
                    named.insert_optional(name, Some(expr));
                }

                Ok(None) => {
                    tail.restart();
                    named.insert_optional(name, None);
                }
            },
        };
    }

    trace_remaining("after named", tail.clone(), source);

    let mut positional = vec![];

    for arg in &config.positional {
        trace!("Processing positional {:?}", arg);

        match arg {
            PositionalType::Mandatory(..) => {
                if tail.len() == 0 {
                    return Err(ShellError::argument_error(
                        config.name.clone(),
                        ArgumentError::MissingMandatoryPositional(arg.name().to_string()),
                        command_span,
                    ));
                }
            }

            PositionalType::Optional(..) => {
                if tail.len() == 0 {
                    break;
                }
            }
        }

        let result = hir::baseline_parse_next_expr(tail, context, source, arg.syntax_type())?;

        positional.push(result);
    }

    trace_remaining("after positional", tail.clone(), source);

    if let Some(syntax_type) = config.rest_positional {
        let remainder = baseline_parse_tokens(tail, context, source, syntax_type)?;
        positional.extend(remainder);
    }

    trace_remaining("after rest", tail.clone(), source);

    trace!("Constructed positional={:?} named={:?}", positional, named);

    let positional = match positional {
        positional if positional.len() == 0 => None,
        positional => Some(positional),
    };

    // TODO: Error if extra unconsumed positional arguments

    let named = match named {
        named if named.named.is_empty() => None,
        named => Some(named),
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
    tokens: &mut hir::TokensIterator<'a>,
    source: &Text,
    span: Span,
) -> Result<(usize, Tagged<Flag>), ShellError> {
    let flag = tokens.extract(|t| t.as_flag(name, source));

    match flag {
        None => Err(ShellError::argument_error(
            config.name.clone(),
            ArgumentError::MissingMandatoryFlag(name.to_string()),
            span,
        )),

        Some((pos, flag)) => {
            tokens.remove(pos);
            Ok((pos, flag))
        }
    }
}

fn extract_optional(
    name: &str,
    tokens: &mut hir::TokensIterator<'a>,
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

pub fn trace_remaining(desc: &'static str, tail: hir::TokensIterator<'a>, source: &Text) {
    trace!(
        "{} = {:?}",
        desc,
        itertools::join(
            tail.debug_remaining()
                .iter()
                .map(|i| format!("%{:?}%", i.debug(source))),
            " "
        )
    );
}
