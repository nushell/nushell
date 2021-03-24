use crate::evaluate::expr::run_expression_block;
use crate::evaluate::internal::run_internal_command;
use crate::evaluation_context::EvaluationContext;
use async_recursion::async_recursion;
use nu_errors::ShellError;
use nu_parser::ParserScope;
use nu_protocol::hir::{Block, ClassifiedCommand, Expression, InternalCommand, Pipeline};
use nu_protocol::UntaggedValue;
use nu_source::Span;
use nu_stream::InputStream;
use std::sync::atomic::Ordering;

/// checks exit condition, returning with Err or Ok from current function if exit condition is met
/// Otherwise does nothing
/// $output is an InputStream
macro_rules! check_exit_condition {
    ($output:ident, $ctx:ident) => {{
        //Check wether we need to exit
        if let Some(err) = $ctx.get_errors().get(0) {
            $ctx.clear_errors();
            return Err(err.clone());
        }
        if $ctx.ctrl_c.swap(false, Ordering::SeqCst) {
            // This early return doesn't return the result
            // we have so far, but breaking out of this loop
            // causes lifetime issues. A future contribution
            // could attempt to return the current output.
            // https://github.com/nushell/nushell/pull/2830#discussion_r550319687
            return Ok($output);
        }
    }};
}

#[async_recursion]
pub async fn run_block(
    block: &Block,
    ctx: &EvaluationContext,
    mut input: InputStream,
) -> Result<InputStream, ShellError> {
    for (_, definition) in block.definitions.iter() {
        ctx.scope.add_definition(definition.clone());
    }

    //We need to return the output stream of the last pipeline in the last group.
    //Other last pipelines in each group get printed

    //So we handle groups[0..-1] (printing last pipeline)
    if block.block.len() > 1 {
        for group in &block.block[..block.block.len() - 1] {
            let pipe_len = group.pipelines.len();

            if pipe_len > 1 {
                //Don't print values of intermediate pipelines
                let output = run_pipelines(&group.pipelines[..pipe_len - 1], input, ctx).await?;

                check_exit_condition!(output, ctx);

                //Input consumed from pipe. Set it to empty, so rust compiler doesn't cry because of
                //moved var
                input = InputStream::empty();
            }

            if let Some(last_pipe) = group.pipelines.last() {
                let output = run_pipeline(last_pipe, ctx, input).await?;

                check_exit_condition!(output, ctx);

                print_stream(output, ctx).await?;

                //Input consumed from pipe. Set it to empty, so rust compiler doesn't cry because of
                //moved var
                input = InputStream::empty();
            }
        }
    }

    //And the last group gets handled special (returning last pipeline output)
    let mut output = InputStream::empty();
    if let Some(last_group) = block.block.last() {
        let pipe_len = last_group.pipelines.len();

        if pipe_len > 1 {
            //Don't print values of intermediate pipelines
            let output = run_pipelines(&last_group.pipelines[..pipe_len - 1], input, ctx).await?;

            check_exit_condition!(output, ctx);

            //Input consumed from pipe. Set it to empty, so rust compiler doesn't cry because of
            //moved var
            input = InputStream::empty();
        }

        if let Some(last_pipe) = last_group.pipelines.last() {
            output = run_pipeline(last_pipe, ctx, input).await?;
            //Last output gets returned. No printing
        }
    }

    Ok(output)
}

async fn print_stream(output: InputStream, ctx: &EvaluationContext) -> Result<(), ShellError> {
    let autoview = InternalCommand::new("autoview".to_string(), Span::unknown(), Span::unknown());
    run_internal_command(autoview, output, ctx)
        .await
        .map(|_| ()) //autoviews output stream is empty
}

async fn run_pipelines(
    pipelines: &[Pipeline],
    mut input: InputStream,
    ctx: &EvaluationContext,
) -> Result<InputStream, ShellError> {
    let mut output = InputStream::empty();
    for pipeline in pipelines {
        output = run_pipeline(pipeline, ctx, input).await?;

        // Only first pipeline of first block might have input, all other inputs are empty
        input = InputStream::empty();

        check_exit_condition!(output, ctx);
    }
    Ok(output)
}

#[async_recursion]
async fn run_pipeline(
    commands: &Pipeline,
    ctx: &EvaluationContext,
    mut input: InputStream,
) -> Result<InputStream, ShellError> {
    for item in commands.list.clone() {
        input = match item {
            ClassifiedCommand::Dynamic(call) => {
                let mut args = vec![];
                if let Some(positional) = call.positional {
                    for pos in &positional {
                        let result = run_expression_block(pos, ctx).await?.into_vec().await;
                        args.push(result);
                    }
                }

                match &call.head.expr {
                    Expression::Block(block) => {
                        ctx.scope.enter_scope();
                        for (param, value) in block.params.positional.iter().zip(args.iter()) {
                            ctx.scope.add_var(param.0.name(), value[0].clone());
                        }
                        let result = run_block(&block, ctx, input).await;
                        ctx.scope.exit_scope();

                        let result = result?;
                        return Ok(result);
                    }
                    Expression::Variable(v, span) => {
                        if let Some(value) = ctx.scope.get_var(v) {
                            match &value.value {
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
                                    let result = run_block(&captured_block.block, ctx, input).await;
                                    ctx.scope.exit_scope();

                                    let result = result?;
                                    return Ok(result);
                                }
                                _ => {
                                    return Err(ShellError::labeled_error("Dynamic commands must start with a block (or variable pointing to a block)", "needs to be a block", call.head.span));
                                }
                            }
                        } else {
                            return Err(ShellError::labeled_error(
                                "Variable not found",
                                "variable not found",
                                span,
                            ));
                        }
                    }
                    _ => {
                        return Err(ShellError::labeled_error("Dynamic commands must start with a block (or variable pointing to a block)", "needs to be a block", call.head.span));
                    }
                }
            }

            ClassifiedCommand::Expr(expr) => run_expression_block(&*expr, ctx).await?,

            ClassifiedCommand::Error(err) => return Err(err.into()),

            ClassifiedCommand::Internal(left) => run_internal_command(left, input, ctx).await?,
        };

        check_exit_condition!(input, ctx);
    }

    Ok(input)
}
