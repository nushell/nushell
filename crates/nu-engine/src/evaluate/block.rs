use crate::evaluate::expr::run_expression_block;
use crate::evaluate::internal::run_internal_command;
use crate::evaluation_context::EvaluationContext;
use nu_errors::ShellError;
use nu_parser::ParserScope;
use nu_protocol::hir::{
    Block, Call, ClassifiedCommand, Expression, ExternalRedirection, Pipeline, SpannedExpression,
    Synthetic,
};
use nu_protocol::{UntaggedValue, Value};
use nu_source::{Span, Tag};
use nu_stream::{InputStream, OutputStream};
use std::sync::atomic::Ordering;

pub fn run_block(
    block: &Block,
    ctx: &EvaluationContext,
    mut input: InputStream,
    external_redirection: ExternalRedirection,
) -> Result<OutputStream, ShellError> {
    let mut output: Result<InputStream, ShellError> = Ok(OutputStream::empty());
    for (_, definition) in block.definitions.iter() {
        ctx.scope.add_definition(definition.clone());
    }

    let num_groups = block.block.len();
    for (group_num, group) in block.block.iter().enumerate() {
        let num_pipelines = group.pipelines.len();
        for (pipeline_num, pipeline) in group.pipelines.iter().enumerate() {
            match output {
                Ok(inp) if inp.is_empty() => {}
                Ok(inp) => {
                    // Run autoview on the values we've seen so far
                    // We may want to make this configurable for other kinds of hosting
                    if let Some(autoview) = ctx.get_command("autoview") {
                        let mut output_stream = match ctx.run_command(
                            autoview,
                            Tag::unknown(),
                            Call::new(
                                Box::new(SpannedExpression::new(
                                    Expression::Synthetic(Synthetic::String("autoview".into())),
                                    Span::unknown(),
                                )),
                                Span::unknown(),
                            ),
                            inp,
                        ) {
                            Ok(x) => x,
                            Err(e) => {
                                return Err(e);
                            }
                        };
                        match output_stream.next() {
                            Some(Value {
                                value: UntaggedValue::Error(e),
                                ..
                            }) => {
                                return Err(e);
                            }
                            Some(_item) => {
                                if let Some(err) = ctx.get_errors().get(0) {
                                    ctx.clear_errors();
                                    return Err(err.clone());
                                }
                                if ctx.ctrl_c().load(Ordering::SeqCst) {
                                    return Ok(InputStream::empty());
                                }
                            }
                            None => {
                                if let Some(err) = ctx.get_errors().get(0) {
                                    ctx.clear_errors();
                                    return Err(err.clone());
                                }
                            }
                        }
                    } else {
                        let _: Vec<_> = inp.collect();
                    }
                }
                Err(e) => {
                    return Err(e);
                }
            }
            output = Ok(OutputStream::empty());

            match output {
                Ok(inp) if inp.is_empty() => {}
                Ok(mut output_stream) => {
                    match output_stream.next() {
                        Some(Value {
                            value: UntaggedValue::Error(e),
                            ..
                        }) => {
                            return Err(e);
                        }
                        Some(_item) => {
                            if let Some(err) = ctx.get_errors().get(0) {
                                ctx.clear_errors();
                                return Err(err.clone());
                            }
                            if ctx.ctrl_c().load(Ordering::SeqCst) {
                                // This early return doesn't return the result
                                // we have so far, but breaking out of this loop
                                // causes lifetime issues. A future contribution
                                // could attempt to return the current output.
                                // https://github.com/nushell/nushell/pull/2830#discussion_r550319687
                                return Ok(InputStream::empty());
                            }
                        }
                        None => {
                            if let Some(err) = ctx.get_errors().get(0) {
                                ctx.clear_errors();
                                return Err(err.clone());
                            }
                        }
                    }
                }
                Err(e) => {
                    return Err(e);
                }
            }
            output = if group_num == (num_groups - 1) && pipeline_num == (num_pipelines - 1) {
                // we're at the end of the block, so use the given external redirection
                run_pipeline(pipeline, ctx, input, external_redirection)
            } else {
                // otherwise, we're in the middle of the block, so use a default redirection
                run_pipeline(pipeline, ctx, input, ExternalRedirection::None)
            };

            input = OutputStream::empty();
        }
    }

    output
}

fn run_pipeline(
    commands: &Pipeline,
    ctx: &EvaluationContext,
    mut input: InputStream,
    external_redirection: ExternalRedirection,
) -> Result<OutputStream, ShellError> {
    let num_commands = commands.list.len();
    for (command_num, command) in commands.list.iter().enumerate() {
        input = match command {
            ClassifiedCommand::Dynamic(call) => {
                let mut args = vec![];
                if let Some(positional) = &call.positional {
                    for pos in positional {
                        let result = run_expression_block(pos, ctx)?.into_vec();
                        args.push(result);
                    }
                }

                let block = run_expression_block(&call.head, ctx)?.into_vec();

                if block.len() != 1 {
                    return Err(ShellError::labeled_error(
                        "Dynamic commands must start with a block",
                        "needs to be a block",
                        call.head.span,
                    ));
                }

                match &block[0].value {
                    UntaggedValue::Block(captured_block) => {
                        ctx.scope.enter_scope();
                        ctx.scope.add_vars(&captured_block.captured.entries);
                        for (param, value) in captured_block
                            .block
                            .params
                            .positional
                            .iter()
                            .zip(args.iter())
                        {
                            ctx.scope.add_var(param.0.name(), value[0].clone());
                        }

                        let result =
                            run_block(&captured_block.block, ctx, input, external_redirection);
                        ctx.scope.exit_scope();

                        result?
                    }
                    _ => {
                        return Err(ShellError::labeled_error("Dynamic commands must start with a block (or variable pointing to a block)", "needs to be a block", call.head.span));
                    }
                }
            }

            ClassifiedCommand::Expr(expr) => run_expression_block(&*expr, ctx)?,

            ClassifiedCommand::Error(err) => return Err(err.clone().into()),

            ClassifiedCommand::Internal(left) => {
                if command_num == (num_commands - 1) {
                    let mut left = left.clone();
                    left.args.external_redirection = external_redirection;
                    run_internal_command(&left, ctx, input)?
                } else {
                    run_internal_command(left, ctx, input)?
                }
            }
        };
    }

    Ok(input)
}
