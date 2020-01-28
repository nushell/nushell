use crate::hir::syntax_shape::{
    BackoffColoringMode, ExpandSyntax, MaybeSpaceShape, MaybeWhitespaceEof,
};
use crate::hir::SpannedExpression;
use crate::TokensIterator;
use crate::{
    hir::{self, NamedArguments},
    Flag,
};
use log::trace;
use nu_errors::{ArgumentError, ParseError};
use nu_protocol::{NamedType, PositionalType, Signature, SyntaxShape};
use nu_source::{HasFallibleSpan, HasSpan, PrettyDebugWithSource, Span, Spanned, SpannedItem};

type OptionalHeadTail = (Option<Vec<hir::SpannedExpression>>, Option<NamedArguments>);

pub fn parse_command_tail(
    config: &Signature,
    tail: &mut TokensIterator,
    command_span: Span,
) -> Result<Option<OptionalHeadTail>, ParseError> {
    let mut named = NamedArguments::new();
    let mut found_error: Option<ParseError> = None;
    let mut rest_signature = config.clone();

    trace!(target: "nu::parse::trace_remaining", "");

    trace_remaining("nodes", &tail);

    for (name, kind) in &config.named {
        trace!(target: "nu::parse::trace_remaining", "looking for {} : {:?}", name, kind);

        tail.move_to(0);

        match &kind.0 {
            NamedType::Switch => {
                let switch = extract_switch(name, tail);

                match switch {
                    None => named.insert_switch(name, None),
                    Some((_, flag)) => {
                        named.insert_switch(name, Some(*flag));
                        rest_signature.remove_named(name);
                        tail.color_shape(flag.color(flag.span));
                    }
                }
            }
            NamedType::Mandatory(syntax_type) => {
                match extract_mandatory(config, name, tail, command_span) {
                    Err(err) => {
                        // remember this error, but continue coloring
                        found_error = Some(err);
                    }
                    Ok((pos, flag)) => {
                        let result = expand_flag(tail, *syntax_type, flag, pos);

                        tail.move_to(0);

                        match result {
                            Ok(expr) => {
                                named.insert_mandatory(name, expr);
                                rest_signature.remove_named(name);
                            }
                            Err(_) => {
                                found_error = Some(ParseError::argument_error(
                                    config.name.clone().spanned(flag.span),
                                    ArgumentError::MissingValueForName(name.to_string()),
                                ))
                            }
                        }
                    }
                }
            }
            NamedType::Optional(syntax_type) => {
                match extract_optional(name, tail) {
                    Err(err) => {
                        // remember this error, but continue coloring
                        found_error = Some(err);
                    }
                    Ok(Some((pos, flag))) => {
                        let result = expand_flag(tail, *syntax_type, flag, pos);

                        tail.move_to(0);

                        match result {
                            Ok(expr) => {
                                named.insert_optional(name, Some(expr));
                                rest_signature.remove_named(name);
                            }
                            Err(_) => {
                                found_error = Some(ParseError::argument_error(
                                    config.name.clone().spanned(flag.span),
                                    ArgumentError::MissingValueForName(name.to_string()),
                                ))
                            }
                        }
                    }

                    Ok(None) => {
                        named.insert_optional(name, None);
                    }
                }
            }
        };
    }

    trace_remaining("after named", &tail);

    let mut positional = vec![];

    match continue_parsing_positionals(&config, tail, &mut rest_signature, command_span) {
        Ok(positionals) => {
            positional = positionals;
        }
        Err(reason) => {
            if found_error.is_none() && !tail.source().contains("help") {
                found_error = Some(reason);
            }
        }
    }

    trace_remaining("after positional", &tail);

    if let Some((syntax_type, _)) = config.rest_positional {
        let mut out = vec![];

        loop {
            if found_error.is_some() {
                break;
            }

            tail.move_to(0);

            trace_remaining("start rest", &tail);
            eat_any_whitespace(tail);
            trace_remaining("after whitespace", &tail);

            if tail.at_end() {
                break;
            }

            match tail.expand_syntax(syntax_type) {
                Err(err) => found_error = Some(err),
                Ok(next) => out.push(next),
            };
        }

        positional.extend(out);
    }

    eat_any_whitespace(tail);

    // Consume any remaining tokens with backoff coloring mode
    tail.expand_infallible(BackoffColoringMode::new(rest_signature.allowed()));

    // This is pretty dubious, but it works. We should look into a better algorithm that doesn't end up requiring
    // this solution.
    tail.sort_shapes();

    if let Some(err) = found_error {
        return Err(err);
    }

    trace_remaining("after rest", &tail);

    trace!(target: "nu::parse::trace_remaining", "Constructed positional={:?} named={:?}", positional, named);

    let positional = if positional.is_empty() {
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

    trace!(target: "nu::parse::trace_remaining", "Normalized positional={:?} named={:?}", positional, named);

    Ok(Some((positional, named)))
}

pub fn continue_parsing_positionals(
    config: &Signature,
    tail: &mut TokensIterator,
    rest_signature: &mut Signature,
    command_span: Span,
) -> Result<Vec<SpannedExpression>, ParseError> {
    let mut positional = vec![];

    for arg in &config.positional {
        trace!(target: "nu::parse::trace_remaining", "Processing positional {:?}", arg);

        tail.move_to(0);

        let result = expand_spaced_expr(arg.0.syntax_type(), tail);

        match result {
            Err(_) => match &arg.0 {
                PositionalType::Mandatory(..) => {
                    return Err(ParseError::argument_error(
                        config.name.clone().spanned(command_span),
                        ArgumentError::MissingMandatoryPositional(arg.0.name().to_string()),
                    ))
                }
                PositionalType::Optional(..) => {
                    if tail.expand_syntax(MaybeWhitespaceEof).is_ok() {
                        break;
                    }
                }
            },
            Ok(result) => {
                rest_signature.shift_positional();
                positional.push(result);
            }
        }
    }

    Ok(positional)
}

fn eat_any_whitespace(tail: &mut TokensIterator) {
    loop {
        match tail.expand_infallible(MaybeSpaceShape) {
            None => break,
            Some(_) => continue,
        }
    }
}

fn expand_flag(
    token_nodes: &mut TokensIterator,
    syntax_type: SyntaxShape,
    flag: Spanned<Flag>,
    pos: usize,
) -> Result<SpannedExpression, ()> {
    token_nodes.color_shape(flag.color(flag.span));

    let result = token_nodes.atomic_parse(|token_nodes| {
        token_nodes.move_to(pos);

        if token_nodes.at_end() {
            return Err(ParseError::unexpected_eof("flag", Span::unknown()));
        }

        let expr = expand_spaced_expr(syntax_type, token_nodes)?;

        Ok(expr)
    });

    let expr = result.map_err(|_| ())?;
    Ok(expr)
}

fn expand_spaced_expr<
    T: HasFallibleSpan + PrettyDebugWithSource + Clone + std::fmt::Debug + 'static,
>(
    syntax: impl ExpandSyntax<Output = Result<T, ParseError>>,
    token_nodes: &mut TokensIterator,
) -> Result<T, ParseError> {
    token_nodes.atomic_parse(|token_nodes| {
        token_nodes.expand_infallible(MaybeSpaceShape);
        token_nodes.expand_syntax(syntax)
    })
}

fn extract_switch(
    name: &str,
    tokens: &mut hir::TokensIterator<'_>,
) -> Option<(usize, Spanned<Flag>)> {
    let source = tokens.source();
    tokens.extract(|t| t.as_flag(name, &source).map(|flag| flag.spanned(t.span())))
}

fn extract_mandatory(
    config: &Signature,
    name: &str,
    tokens: &mut hir::TokensIterator<'_>,
    span: Span,
) -> Result<(usize, Spanned<Flag>), ParseError> {
    let source = tokens.source();
    let flag = tokens.extract(|t| t.as_flag(name, &source).map(|flag| flag.spanned(t.span())));

    match flag {
        None => Err(ParseError::argument_error(
            config.name.clone().spanned(span),
            ArgumentError::MissingMandatoryFlag(name.to_string()),
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
) -> Result<Option<(usize, Spanned<Flag>)>, ParseError> {
    let source = tokens.source();
    let flag = tokens.extract(|t| t.as_flag(name, &source).map(|flag| flag.spanned(t.span())));

    match flag {
        None => Ok(None),
        Some((pos, flag)) => {
            tokens.remove(pos);
            Ok(Some((pos, flag)))
        }
    }
}

pub fn trace_remaining(desc: &'static str, tail: &hir::TokensIterator<'_>) {
    let offset = tail.clone().span_at_cursor();
    let source = tail.source();

    trace!(
        target: "nu::parse::trace_remaining",
        "{} = {}",
        desc,
        itertools::join(
            tail.debug_remaining()
                .iter()
                .map(|val| {
                    if val.span().start() == offset.start() {
                        format!("<|> %{}%", val.debug(&source))
                    } else {
                        format!("%{}%", val.debug(&source))
                    }
                }),
            " "
        )
    );
}
