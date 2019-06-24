use crate::errors::ShellError;
use crate::parser::registry::{CommandConfig, CommandRegistry, NamedType};
use crate::parser::{baseline_parse_tokens, CallNode, Spanned};
use crate::parser::{
    hir::{self, NamedArguments},
    Flag, RawToken, TokenNode,
};
use crate::Text;
use log::trace;

pub fn parse_command(
    config: &CommandConfig,
    registry: &dyn CommandRegistry,
    call: &Spanned<CallNode>,
    source: &Text,
) -> Result<hir::Call, ShellError> {
    let Spanned { item: call, .. } = call;

    trace!("Processing {:?}", config);

    let head = parse_command_head(call.head())?;

    let children: Option<Vec<TokenNode>> = call.children().as_ref().map(|nodes| {
        nodes
            .iter()
            .cloned()
            .filter(|node| match node {
                TokenNode::Whitespace(_) => false,
                _ => true,
            })
            .collect()
    });

    match parse_command_tail(&config, registry, children, source)? {
        None => Ok(hir::Call::new(Box::new(head), None, None)),
        Some((positional, named)) => Ok(hir::Call::new(Box::new(head), positional, named)),
    }
}

fn parse_command_head(head: &TokenNode) -> Result<hir::Expression, ShellError> {
    match head {
        TokenNode::Token(
            spanned @ Spanned {
                item: RawToken::Bare,
                ..
            },
        ) => Ok(spanned.map(|_| hir::RawExpression::Literal(hir::Literal::Bare))),

        TokenNode::Token(Spanned {
            item: RawToken::String(inner_span),
            span,
        }) => Ok(Spanned::from_item(
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
    config: &CommandConfig,
    registry: &dyn CommandRegistry,
    tail: Option<Vec<TokenNode>>,
    source: &Text,
) -> Result<Option<(Option<Vec<hir::Expression>>, Option<NamedArguments>)>, ShellError> {
    let mut tail = match tail {
        None => return Ok(None),
        Some(tail) => tail,
    };

    let mut named = NamedArguments::new();

    for (name, kind) in config.named() {
        trace!("looking for {} : {:?}", name, kind);

        match kind {
            NamedType::Switch => {
                let (rest, flag) = extract_switch(name, tail, source);

                tail = rest;

                named.insert_switch(name, flag);
            }
            NamedType::Mandatory(kind) => match extract_mandatory(name, tail, source) {
                Err(err) => return Err(err), // produce a correct diagnostic
                Ok((rest, pos, _flag)) => {
                    let (expr, rest) = hir::baseline_parse_next_expr(
                        &rest[pos..],
                        registry,
                        source,
                        kind.to_coerce_hint(),
                    )?;
                    tail = rest.to_vec();

                    named.insert_mandatory(name, expr);
                }
            },
            NamedType::Optional(kind) => match extract_optional(name, tail, source) {
                Err(err) => return Err(err), // produce a correct diagnostic
                Ok((rest, Some((pos, _flag)))) => {
                    let (expr, rest) = hir::baseline_parse_next_expr(
                        &rest[pos..],
                        registry,
                        source,
                        kind.to_coerce_hint(),
                    )?;
                    tail = rest.to_vec();

                    named.insert_optional(name, Some(expr));
                }

                Ok((rest, None)) => {
                    tail = rest;

                    named.insert_optional(name, None);
                }
            },
        };
    }

    let mut positional = vec![];
    let mandatory = config.mandatory_positional();

    for arg in mandatory {
        if tail.len() == 0 {
            return Err(ShellError::unimplemented("Missing mandatory argument"));
        }

        let (result, rest) =
            hir::baseline_parse_next_expr(&tail, registry, source, arg.to_coerce_hint())?;

        positional.push(result);

        tail = rest.to_vec();
    }

    let optional = config.optional_positional();

    for arg in optional {
        if tail.len() == 0 {
            break;
        }

        let (result, rest) =
            hir::baseline_parse_next_expr(&tail, registry, source, arg.to_coerce_hint())?;

        positional.push(result);

        tail = rest.to_vec();
    }

    // TODO: Only do this if rest params are specified
    let remainder = baseline_parse_tokens(&tail, registry, source)?;
    positional.extend(remainder);

    trace!("Constructed positional={:?} named={:?}", positional, named);

    let positional = match positional {
        positional if positional.len() == 0 => None,
        positional => Some(positional),
    };

    let named = match named {
        named if named.named.is_empty() => None,
        named => Some(named),
    };

    trace!("Normalized positional={:?} named={:?}", positional, named);

    Ok(Some((positional, named)))
}

fn extract_switch(
    name: &str,
    mut tokens: Vec<TokenNode>,
    source: &Text,
) -> (Vec<TokenNode>, Option<Flag>) {
    let pos = tokens
        .iter()
        .enumerate()
        .filter_map(|(i, t)| t.as_flag(name, source).map(|f| (i, f)))
        .nth(0);

    match pos {
        None => (tokens, None),
        Some((pos, flag)) => {
            tokens.remove(pos);
            (tokens, Some(*flag))
        }
    }
}

fn extract_mandatory(
    name: &str,
    mut tokens: Vec<TokenNode>,
    source: &Text,
) -> Result<(Vec<TokenNode>, usize, Flag), ShellError> {
    let pos = tokens
        .iter()
        .enumerate()
        .filter_map(|(i, t)| t.as_flag(name, source).map(|f| (i, f)))
        .nth(0);

    match pos {
        None => Err(ShellError::unimplemented(
            "Better error: mandatory flags must be present",
        )),
        Some((pos, flag)) => {
            if tokens.len() <= pos {
                return Err(ShellError::unimplemented(
                    "Better errors: mandatory flags must be followed by values",
                ));
            }

            tokens.remove(pos);

            Ok((tokens, pos, *flag))
        }
    }
}

fn extract_optional(
    name: &str,
    mut tokens: Vec<TokenNode>,
    source: &Text,
) -> Result<(Vec<TokenNode>, Option<(usize, Flag)>), ShellError> {
    let pos = tokens
        .iter()
        .enumerate()
        .filter_map(|(i, t)| t.as_flag(name, source).map(|f| (i, f)))
        .nth(0);

    match pos {
        None => Ok((tokens, None)),
        Some((pos, flag)) => {
            if tokens.len() <= pos {
                return Err(ShellError::unimplemented(
                    "Better errors: optional flags must be followed by values",
                ));
            }

            tokens.remove(pos);

            Ok((tokens, Some((pos, *flag))))
        }
    }
}
