use crate::errors::{ArgumentError, ParseError};
use crate::parser::hir::syntax_shape::{
    color_fallible_syntax, color_syntax, expand_expr, flat_shape::FlatShape, spaced,
    BackoffColoringMode, ColorSyntax, MaybeSpaceShape,
};
use crate::parser::registry::{NamedType, PositionalType, Signature};
use crate::parser::TokensIterator;
use crate::parser::{
    hir::{self, ExpandContext, NamedArguments},
    Flag,
};
use crate::traits::ToDebug;
use crate::{Span, Spanned, SpannedItem, Text};
use log::trace;

pub fn parse_command_tail(
    config: &Signature,
    context: &ExpandContext,
    tail: &mut TokensIterator,
    command_span: Span,
) -> Result<Option<(Option<Vec<hir::Expression>>, Option<NamedArguments>)>, ParseError> {
    let mut named = NamedArguments::new();
    trace_remaining("nodes", &tail, context.source());

    for (name, kind) in &config.named {
        trace!(target: "nu::parse", "looking for {} : {:?}", name, kind);

        match &kind.0 {
            NamedType::Switch => {
                let flag = extract_switch(name, tail, context.source());

                named.insert_switch(name, flag);
            }
            NamedType::Mandatory(syntax_type) => {
                match extract_mandatory(config, name, tail, context.source(), command_span) {
                    Err(err) => return Err(err), // produce a correct diagnostic
                    Ok((pos, flag)) => {
                        tail.move_to(pos);

                        if tail.at_end() {
                            return Err(ParseError::argument_error(
                                config.name.clone().spanned(flag.span),
                                ArgumentError::MissingValueForName(name.to_string()),
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
                            return Err(ParseError::argument_error(
                                config.name.clone().spanned(flag.span),
                                ArgumentError::MissingValueForName(name.to_string()),
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

    trace_remaining("after named", &tail, context.source());

    let mut positional = vec![];

    for arg in &config.positional {
        trace!(target: "nu::parse", "Processing positional {:?}", arg);

        match &arg.0 {
            PositionalType::Mandatory(..) => {
                if tail.at_end_possible_ws() {
                    return Err(ParseError::argument_error(
                        config.name.clone().spanned(command_span),
                        ArgumentError::MissingMandatoryPositional(arg.0.name().to_string()),
                    ));
                }
            }

            PositionalType::Optional(..) => {
                if tail.at_end_possible_ws() {
                    break;
                }
            }
        }

        let result = expand_expr(&spaced(arg.0.syntax_type()), tail, context)?;

        positional.push(result);
    }

    trace_remaining("after positional", &tail, context.source());

    if let Some((syntax_type, _)) = config.rest_positional {
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

    trace_remaining("after rest", &tail, context.source());

    trace!(target: "nu::parse", "Constructed positional={:?} named={:?}", positional, named);

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

    trace!(target: "nu::parse", "Normalized positional={:?} named={:?}", positional, named);

    Ok(Some((positional, named)))
}

#[derive(Debug)]
struct ColoringArgs {
    vec: Vec<Option<Vec<Spanned<FlatShape>>>>,
}

impl ColoringArgs {
    fn new(len: usize) -> ColoringArgs {
        let vec = vec![None; len];
        ColoringArgs { vec }
    }

    fn insert(&mut self, pos: usize, shapes: Vec<Spanned<FlatShape>>) {
        self.vec[pos] = Some(shapes);
    }

    fn spread_shapes(self, shapes: &mut Vec<Spanned<FlatShape>>) {
        for item in self.vec {
            match item {
                None => {}
                Some(vec) => {
                    shapes.extend(vec);
                }
            }
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct CommandTailShape;

#[cfg(not(coloring_in_tokens))]
impl ColorSyntax for CommandTailShape {
    type Info = ();
    type Input = Signature;

    fn color_syntax<'a, 'b>(
        &self,
        signature: &Signature,
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
        shapes: &mut Vec<Spanned<FlatShape>>,
    ) -> Self::Info {
        let mut args = ColoringArgs::new(token_nodes.len());
        for (name, kind) in &signature.named {
            trace!(target: "nu::color_syntax", "looking for {} : {:?}", name, kind);

            match &kind.0 {
                NamedType::Switch => {
                    match token_nodes.extract(|t| t.as_flag(name, context.source())) {
                        Some((pos, flag)) => args.insert(pos, vec![flag.color()]),
                        None => {}
                    }
                }
                NamedType::Mandatory(syntax_type) => {
                    match extract_mandatory(
                        signature,
                        name,
                        token_nodes,
                        context.source(),
                        Span::unknown(),
                    ) {
                        Err(_) => {
                            // The mandatory flag didn't exist at all, so there's nothing to color
                        }
                        Ok((pos, flag)) => {
                            let mut shapes = vec![flag.color()];
                            token_nodes.move_to(pos);

                            if token_nodes.at_end() {
                                args.insert(pos, shapes);
                                token_nodes.restart();
                                continue;
                            }

                            // We can live with unmatched syntax after a mandatory flag
                            let _ = token_nodes.atomic(|token_nodes| {
                                color_syntax(&MaybeSpaceShape, token_nodes, context, &mut shapes);

                                // If the part after a mandatory flag isn't present, that's ok, but we
                                // should roll back any whitespace we chomped
                                color_fallible_syntax(
                                    syntax_type,
                                    token_nodes,
                                    context,
                                    &mut shapes,
                                )
                            });

                            args.insert(pos, shapes);
                            token_nodes.restart();
                        }
                    }
                }
                NamedType::Optional(syntax_type) => {
                    match extract_optional(name, token_nodes, context.source()) {
                        Err(_) => {
                            // The optional flag didn't exist at all, so there's nothing to color
                        }
                        Ok(Some((pos, flag))) => {
                            let mut shapes = vec![flag.color()];
                            token_nodes.move_to(pos);

                            if token_nodes.at_end() {
                                args.insert(pos, shapes);
                                token_nodes.restart();
                                continue;
                            }

                            // We can live with unmatched syntax after an optional flag
                            let _ = token_nodes.atomic(|token_nodes| {
                                color_syntax(&MaybeSpaceShape, token_nodes, context, &mut shapes);

                                // If the part after a mandatory flag isn't present, that's ok, but we
                                // should roll back any whitespace we chomped
                                color_fallible_syntax(
                                    syntax_type,
                                    token_nodes,
                                    context,
                                    &mut shapes,
                                )
                            });

                            args.insert(pos, shapes);
                            token_nodes.restart();
                        }

                        Ok(None) => {
                            token_nodes.restart();
                        }
                    }
                }
            };
        }

        for arg in &signature.positional {
            trace!("Processing positional {:?}", arg);

            match arg.0 {
                PositionalType::Mandatory(..) => {
                    if token_nodes.at_end() {
                        break;
                    }
                }

                PositionalType::Optional(..) => {
                    if token_nodes.at_end() {
                        break;
                    }
                }
            }

            let mut shapes = vec![];
            let pos = token_nodes.pos(false);

            match pos {
                None => break,
                Some(pos) => {
                    // We can live with an unmatched positional argument. Hopefully it will be
                    // matched by a future token
                    let _ = token_nodes.atomic(|token_nodes| {
                        color_syntax(&MaybeSpaceShape, token_nodes, context, &mut shapes);

                        // If no match, we should roll back any whitespace we chomped
                        color_fallible_syntax(
                            &arg.0.syntax_type(),
                            token_nodes,
                            context,
                            &mut shapes,
                        )?;

                        args.insert(pos, shapes);

                        Ok(())
                    });
                }
            }
        }

        if let Some((syntax_type, _)) = signature.rest_positional {
            loop {
                if token_nodes.at_end_possible_ws() {
                    break;
                }

                let pos = token_nodes.pos(false);

                match pos {
                    None => break,
                    Some(pos) => {
                        let mut shapes = vec![];

                        // If any arguments don't match, we'll fall back to backoff coloring mode
                        let result = token_nodes.atomic(|token_nodes| {
                            color_syntax(&MaybeSpaceShape, token_nodes, context, &mut shapes);

                            // If no match, we should roll back any whitespace we chomped
                            color_fallible_syntax(&syntax_type, token_nodes, context, &mut shapes)?;

                            args.insert(pos, shapes);

                            Ok(())
                        });

                        match result {
                            Err(_) => break,
                            Ok(_) => continue,
                        }
                    }
                }
            }
        }

        args.spread_shapes(shapes);

        // Consume any remaining tokens with backoff coloring mode
        color_syntax(&BackoffColoringMode, token_nodes, context, shapes);

        shapes.sort_by(|a, b| a.span.start().cmp(&b.span.start()));
    }
}

#[cfg(coloring_in_tokens)]
impl ColorSyntax for CommandTailShape {
    type Info = ();
    type Input = Signature;

    fn name(&self) -> &'static str {
        "CommandTailShape"
    }

    fn color_syntax<'a, 'b>(
        &self,
        signature: &Signature,
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Self::Info {
        let mut args = ColoringArgs::new(token_nodes.len());
        trace_remaining("nodes", &token_nodes, context.source());

        for (name, kind) in &signature.named {
            trace!(target: "nu::color_syntax", "looking for {} : {:?}", name, kind);

            match &kind.0 {
                NamedType::Switch => {
                    match token_nodes.extract(|t| t.as_flag(name, context.source())) {
                        Some((pos, flag)) => args.insert(pos, vec![flag.color()]),
                        None => {}
                    }
                }
                NamedType::Mandatory(syntax_type) => {
                    match extract_mandatory(
                        signature,
                        name,
                        token_nodes,
                        context.source(),
                        Span::unknown(),
                    ) {
                        Err(_) => {
                            // The mandatory flag didn't exist at all, so there's nothing to color
                        }
                        Ok((pos, flag)) => {
                            let (_, shapes) = token_nodes.atomic_returning_shapes(|token_nodes| {
                                token_nodes.color_shape(flag.color());
                                token_nodes.move_to(pos);

                                if token_nodes.at_end() {
                                    return Ok(());
                                }

                                // We still want to color the flag even if the following tokens don't match, so don't
                                // propagate the error to the parent atomic block if it fails
                                let _ = token_nodes.atomic(|token_nodes| {
                                    // We can live with unmatched syntax after a mandatory flag
                                    color_syntax(&MaybeSpaceShape, token_nodes, context);

                                    // If the part after a mandatory flag isn't present, that's ok, but we
                                    // should roll back any whitespace we chomped
                                    color_fallible_syntax(syntax_type, token_nodes, context)?;

                                    Ok(())
                                });

                                Ok(())
                            });

                            args.insert(pos, shapes);
                            token_nodes.restart();
                        }
                    }
                }
                NamedType::Optional(syntax_type) => {
                    match extract_optional(name, token_nodes, context.source()) {
                        Err(_) => {
                            // The optional flag didn't exist at all, so there's nothing to color
                        }
                        Ok(Some((pos, flag))) => {
                            let (_, shapes) = token_nodes.atomic_returning_shapes(|token_nodes| {
                                token_nodes.color_shape(flag.color());
                                token_nodes.move_to(pos);

                                if token_nodes.at_end() {
                                    return Ok(());
                                }

                                // We still want to color the flag even if the following tokens don't match, so don't
                                // propagate the error to the parent atomic block if it fails
                                let _ = token_nodes.atomic(|token_nodes| {
                                    // We can live with unmatched syntax after a mandatory flag
                                    color_syntax(&MaybeSpaceShape, token_nodes, context);

                                    // If the part after a mandatory flag isn't present, that's ok, but we
                                    // should roll back any whitespace we chomped
                                    color_fallible_syntax(syntax_type, token_nodes, context)?;

                                    Ok(())
                                });

                                Ok(())
                            });

                            args.insert(pos, shapes);
                            token_nodes.restart();
                        }

                        Ok(None) => {
                            token_nodes.restart();
                        }
                    }
                }
            };
        }

        trace_remaining("after named", &token_nodes, context.source());

        for arg in &signature.positional {
            trace!("Processing positional {:?}", arg);

            match &arg.0 {
                PositionalType::Mandatory(..) => {
                    if token_nodes.at_end() {
                        break;
                    }
                }

                PositionalType::Optional(..) => {
                    if token_nodes.at_end() {
                        break;
                    }
                }
            }

            let pos = token_nodes.pos(false);

            match pos {
                None => break,
                Some(pos) => {
                    // We can live with an unmatched positional argument. Hopefully it will be
                    // matched by a future token
                    let (_, shapes) = token_nodes.atomic_returning_shapes(|token_nodes| {
                        color_syntax(&MaybeSpaceShape, token_nodes, context);

                        // If no match, we should roll back any whitespace we chomped
                        color_fallible_syntax(&arg.0.syntax_type(), token_nodes, context)?;

                        Ok(())
                    });

                    args.insert(pos, shapes);
                }
            }
        }

        trace_remaining("after positional", &token_nodes, context.source());

        if let Some((syntax_type, _)) = signature.rest_positional {
            loop {
                if token_nodes.at_end_possible_ws() {
                    break;
                }

                let pos = token_nodes.pos(false);

                match pos {
                    None => break,
                    Some(pos) => {
                        // If any arguments don't match, we'll fall back to backoff coloring mode
                        let (result, shapes) = token_nodes.atomic_returning_shapes(|token_nodes| {
                            color_syntax(&MaybeSpaceShape, token_nodes, context);

                            // If no match, we should roll back any whitespace we chomped
                            color_fallible_syntax(&syntax_type, token_nodes, context)?;

                            Ok(())
                        });

                        args.insert(pos, shapes);

                        match result {
                            Err(_) => break,
                            Ok(_) => continue,
                        }
                    }
                }
            }
        }

        token_nodes.silently_mutate_shapes(|shapes| args.spread_shapes(shapes));

        // Consume any remaining tokens with backoff coloring mode
        color_syntax(&BackoffColoringMode, token_nodes, context);

        // This is pretty dubious, but it works. We should look into a better algorithm that doesn't end up requiring
        // this solution.
        token_nodes.sort_shapes()
    }
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
    span: Span,
) -> Result<(usize, Spanned<Flag>), ParseError> {
    let flag = tokens.extract(|t| t.as_flag(name, source));

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
    source: &Text,
) -> Result<Option<(usize, Spanned<Flag>)>, ParseError> {
    let flag = tokens.extract(|t| t.as_flag(name, source));

    match flag {
        None => Ok(None),
        Some((pos, flag)) => {
            tokens.remove(pos);
            Ok(Some((pos, flag)))
        }
    }
}

pub fn trace_remaining(desc: &'static str, tail: &hir::TokensIterator<'_>, source: &Text) {
    trace!(
        target: "nu::parse",
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
