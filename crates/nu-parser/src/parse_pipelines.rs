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

/// Context for building the RHS pipeline of `$foo |= bar | baz` as `$foo | bar | baz`
pub(crate) struct PipeAssignContext {
    /// The parsed assignment target, inserted before the first RHS command.
    /// Its type is used to parse that command.
    pub lhs: Expression,
    /// The span of `|=`, stored as the pipe before the first RHS command.
    pub op_span: Span,
}

impl PipeAssignContext {
    pub fn ty(&self) -> &Type {
        &self.lhs.ty
    }
}

fn wrap_if_has_in(
    working_set: &mut StateWorkingSet,
    element: PipelineElement,
    input_type: &Type,
) -> PipelineElement {
    if element.has_in_variable(working_set) {
        wrap_element_with_collect(working_set, element, Some(input_type))
    } else {
        element
    }
}

pub fn parse_pipeline(
    working_set: &mut StateWorkingSet,
    pipeline: &LitePipeline,
    input_type: Option<&Type>,
    pipe_assign: Option<&PipeAssignContext>,
) -> Pipeline {
    let (first, rest) = pipeline
        .commands
        .split_first()
        .expect("at this point the pipeline must have at least one element");

    let first = match (pipe_assign, rest) {
        (Some(pipe_assign), _) => {
            let element = parse_pipeline_element(working_set, first, pipe_assign.ty());
            wrap_if_has_in(working_set, element, pipe_assign.ty())
        }
        (None, []) => return parse_builtin_commands(working_set, first, input_type),
        (None, _) => parse_pipeline_element(working_set, first, input_type.unwrap_or(&Type::Any)),
    };

    // the output becomes the input for the next pipeline element
    let mut current_type = first.expr.ty.clone();

    let mut elements = Vec::with_capacity(pipeline.commands.len());
    elements.push(first);

    for command in rest {
        let element = parse_pipeline_element(working_set, command, &current_type);
        let element = wrap_if_has_in(working_set, element, &current_type);

        // the output becomes the input for the next pipeline element
        current_type = element.expr.ty.clone();
        elements.push(element);
    }

    Pipeline { elements }
}

pub fn parse_block(
    working_set: &mut StateWorkingSet,
    tokens: &[Token],
    span: Span,
    scoped: bool,
    is_subexpression: bool,
    input_type: Option<&Type>,
) -> Block {
    parse_block_with_pipe_assign(
        working_set,
        tokens,
        span,
        scoped,
        is_subexpression,
        input_type,
        None,
    )
}

/// separated from [`parse_block`] since the vast majority
/// of callers will pass `None`s for the `pipe_assign` parameter
pub(crate) fn parse_block_with_pipe_assign(
    working_set: &mut StateWorkingSet,
    tokens: &[Token],
    span: Span,
    scoped: bool,
    is_subexpression: bool,
    input_type: Option<&Type>,
    pipe_assign: Option<PipeAssignContext>,
) -> Block {
    let (lite_block, err) = lite_parse(tokens, working_set);
    if let Some(err) = err {
        working_set.error(err);
    }

    trace!("parsing block: {lite_block:?}");

    if scoped {
        working_set.enter_scope();
    }

    let mut block = Block::new_with_capacity(lite_block.block.len());
    block.span = Some(span);
    block.parsed_scoped = scoped;

    if let [first, rest @ ..] = lite_block.block.as_slice() {
        // Pre-declare any definition so that definitions
        // that share the same block can see each other
        if let [lite_command] = first.commands.as_slice()
            && pipe_assign.is_none()
        {
            parse_def_predecl(working_set, lite_command.command_parts());
        }

        for pipeline in rest {
            if let [lite_command] = pipeline.commands.as_slice() {
                parse_def_predecl(working_set, lite_command.command_parts())
            }
        }

        // only the first pipeline receives the block's pipeline input
        let pipeline = parse_pipeline(working_set, first, input_type, pipe_assign.as_ref());
        block.pipelines.push(pipeline);

        for lite_pipeline in rest {
            let pipeline = parse_pipeline(working_set, lite_pipeline, None, None);
            block.pipelines.push(pipeline);
        }
    }

    if let Some(PipeAssignContext { lhs, op_span }) = pipe_assign
        && let Some(pipeline) = block.pipelines.first_mut()
    {
        if let Some(first) = pipeline.elements.first_mut() {
            first.pipe = Some(op_span);
        }

        let element = PipelineElement {
            pipe: None,
            expr: lhs,
            redirection: None,
        };

        pipeline.elements.insert(0, element);
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
        block.parsed_scoped = scoped;
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
