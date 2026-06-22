use log::trace;
use nu_protocol::{ParseError, Span, SyntaxShape, Type, ast::*, engine::StateWorkingSet};
use std::sync::Arc;

use crate::{
    Token,
    lite_parser::{LiteCommand, LitePipeline, LiteRedirection, LiteRedirectionTarget, lite_parse},
    parse_keywords::parse_def_predecl,
    parser::{
        parse_builtin_commands, parse_expression, parse_value, wrap_element_with_collect,
        wrap_expr_with_collect,
    },
    type_check,
};

fn parse_redirection_target(
    working_set: &mut StateWorkingSet,
    target: &LiteRedirectionTarget,
) -> RedirectionTarget {
    match target {
        LiteRedirectionTarget::File {
            connector,
            file,
            append,
        } => RedirectionTarget::File {
            expr: parse_value(working_set, *file, &SyntaxShape::Any),
            append: *append,
            span: *connector,
        },
        LiteRedirectionTarget::Pipe { connector } => RedirectionTarget::Pipe { span: *connector },
    }
}

pub(crate) fn parse_redirection(
    working_set: &mut StateWorkingSet,
    target: &LiteRedirection,
) -> PipelineRedirection {
    match target {
        LiteRedirection::Single { source, target } => PipelineRedirection::Single {
            source: *source,
            target: parse_redirection_target(working_set, target),
        },
        LiteRedirection::Separate { out, err } => PipelineRedirection::Separate {
            out: parse_redirection_target(working_set, out),
            err: parse_redirection_target(working_set, err),
        },
    }
}

pub(crate) fn parse_pipeline_element(
    working_set: &mut StateWorkingSet,
    command: &LiteCommand,
    input_type: Type,
) -> PipelineElement {
    trace!("parsing: pipeline element");

    let expr = parse_expression(working_set, &command.parts, Some(input_type));

    let redirection = command
        .redirection
        .as_ref()
        .map(|r| parse_redirection(working_set, r));

    PipelineElement {
        pipe: command.pipe,
        expr,
        redirection,
    }
}

pub(crate) fn redirecting_builtin_error(
    name: &'static str,
    redirection: &LiteRedirection,
) -> ParseError {
    match redirection {
        LiteRedirection::Single { target, .. } => {
            ParseError::RedirectingBuiltinCommand(name, target.connector(), None)
        }
        LiteRedirection::Separate { out, err } => ParseError::RedirectingBuiltinCommand(
            name,
            out.connector().min(err.connector()),
            Some(out.connector().max(err.connector())),
        ),
    }
}

pub fn parse_pipeline(
    working_set: &mut StateWorkingSet,
    pipeline: &LitePipeline,
    input_type: Option<Type>,
) -> Pipeline {
    let mut input = input_type.unwrap_or(Type::Any);

    if pipeline.commands.len() > 1 {
        // Parse a normal multi command pipeline
        let elements: Vec<_> = pipeline
            .commands
            .iter()
            .enumerate()
            .map(|(index, element)| {
                let element =
                    parse_pipeline_element(working_set, element, std::mem::take(&mut input));
                input = element.expr.ty.clone();
                // Handle $in for pipeline elements beyond the first one
                if index > 0 && element.has_in_variable(working_set) {
                    wrap_element_with_collect(working_set, element)
                } else {
                    element
                }
            })
            .collect();

        Pipeline { elements }
    } else {
        // If there's only one command in the pipeline, this could be a builtin command
        parse_builtin_commands(working_set, &pipeline.commands[0])
    }
}

pub fn parse_block(
    working_set: &mut StateWorkingSet,
    tokens: &[Token],
    span: Span,
    scoped: bool,
    is_subexpression: bool,
) -> Block {
    let (lite_block, err) = lite_parse(tokens, working_set);
    if let Some(err) = err {
        working_set.error(err);
    }

    trace!("parsing block: {lite_block:?}");

    if scoped {
        working_set.enter_scope();
    }

    // Pre-declare any definition so that definitions
    // that share the same block can see each other
    for pipeline in &lite_block.block {
        if pipeline.commands.len() == 1 {
            parse_def_predecl(working_set, pipeline.commands[0].command_parts())
        }
    }

    let mut block = Block::new_with_capacity(lite_block.block.len());
    block.span = Some(span);

    for lite_pipeline in &lite_block.block {
        let pipeline = parse_pipeline(working_set, lite_pipeline, None);
        block.pipelines.push(pipeline);
    }

    // If this is not a subexpression and there are any pipelines where the first element has $in,
    // we can wrap the whole block in collect so that they all reference the same $in
    if !is_subexpression
        && block
            .pipelines
            .iter()
            .flat_map(|pipeline| pipeline.elements.first())
            .any(|element| element.has_in_variable(working_set))
    {
        // Move the block out to prepare it to become a subexpression
        let inner_block = std::mem::take(&mut block);
        block.span = inner_block.span;
        let ty = inner_block.output_type();
        let block_id = working_set.add_block(Arc::new(inner_block));

        // Now wrap it in a Collect expression, and put it in the block as the only pipeline
        let subexpression = Expression::new(working_set, Expr::Subexpression(block_id), span, ty);
        let collect = wrap_expr_with_collect(working_set, subexpression);

        block.pipelines.push(Pipeline {
            elements: vec![PipelineElement {
                pipe: None,
                expr: collect,
                redirection: None,
            }],
        });
    }

    if scoped {
        working_set.exit_scope();
    }

    let errors = type_check::check_block_input_output(working_set, &block);
    if !errors.is_empty() {
        working_set.parse_errors.extend_from_slice(&errors);
    }

    block
}
