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
            expr: parse_value(working_set, *file, &SyntaxShape::Any, None),
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
    input_type: &Type,
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
    input_type: Option<&Type>,
) -> Pipeline {
    match pipeline.commands.as_slice() {
        [] => unreachable!("at this point the pipeline must have at least one element"),
        [single] => parse_builtin_commands(working_set, single, input_type),
        [first, rest @ ..] => {
            let mut current_pipeline_type = input_type.cloned().unwrap_or(Type::Any);

            let mut elements = Vec::new();
            elements.push({
                let element = parse_pipeline_element(working_set, first, &current_pipeline_type);
                // the output becomes the input for the next pipeline element
                current_pipeline_type = element.expr.ty.clone();

                element
            });

            // Parse a normal multi command pipeline
            let rest_elements = rest.iter().map(|element| {
                let input_clone = current_pipeline_type.clone();
                let element = parse_pipeline_element(working_set, element, &current_pipeline_type);
                // the output becomes the input for the next pipeline element
                current_pipeline_type = element.expr.ty.clone();

                // Handle $in for pipeline elements beyond the first one
                if element.has_in_variable(working_set) {
                    wrap_element_with_collect(working_set, element, Some(&input_clone))
                } else {
                    element
                }
            });

            elements.extend(rest_elements);

            Pipeline { elements }
        }
    }
}

pub fn parse_block(
    working_set: &mut StateWorkingSet,
    tokens: &[Token],
    span: Span,
    scoped: bool,
    is_subexpression: bool,
    input_type: Option<&Type>,
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
        if let [lite_command] = pipeline.commands.as_slice() {
            parse_def_predecl(working_set, lite_command.command_parts())
        }
    }

    let mut block = Block::new_with_capacity(lite_block.block.len());
    block.span = Some(span);

    if let [first, rest @ ..] = lite_block.block.as_slice() {
        // only the first pipeline receives the block's pipeline input
        let pipeline = parse_pipeline(working_set, first, input_type);
        block.pipelines.push(pipeline);

        for lite_pipeline in rest {
            let pipeline = parse_pipeline(working_set, lite_pipeline, None);
            block.pipelines.push(pipeline);
        }
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
        let collect = wrap_expr_with_collect(working_set, subexpression, input_type);

        block.pipelines.push(Pipeline {
            elements: vec![PipelineElement {
                pipe: None,
                expr: collect,
                redirection: None,
            }],
        });
    }

    if scoped {
        block.scope_bindings = working_set.snapshot_scope_bindings();
        working_set.exit_scope();
    }

    let errors = type_check::check_block_input_output(working_set, &block);
    working_set.parse_errors.extend(errors);

    block
}
