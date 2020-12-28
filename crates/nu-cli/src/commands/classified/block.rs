use crate::commands::classified::expr::run_expression_block;
use crate::commands::classified::internal::run_internal_command;
use crate::evaluation_context::EvaluationContext;
use crate::prelude::*;
use async_recursion::async_recursion;
use futures::stream::TryStreamExt;
use nu_errors::ShellError;
use nu_protocol::hir::{
    Block, Call, ClassifiedCommand, Expression, Pipeline, SpannedExpression, Synthetic,
};
use nu_protocol::{ReturnSuccess, UntaggedValue, Value};
use nu_stream::InputStream;
use std::sync::atomic::Ordering;

#[async_recursion]
pub async fn run_block(
    block: &Block,
    ctx: &EvaluationContext,
    mut input: InputStream,
) -> Result<InputStream, ShellError> {
    let mut output: Result<InputStream, ShellError> = Ok(InputStream::empty());
    for group in &block.block {
        match output {
            Ok(inp) if inp.is_empty() => {}
            Ok(inp) => {
                // Run autoview on the values we've seen so far
                // We may want to make this configurable for other kinds of hosting
                if let Some(autoview) = ctx.get_command("autoview") {
                    let mut output_stream = ctx
                        .run_command(
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
                        )
                        .await?;
                    loop {
                        match output_stream.try_next().await {
                            Ok(Some(ReturnSuccess::Value(Value {
                                value: UntaggedValue::Error(e),
                                ..
                            }))) => return Err(e),
                            Ok(Some(_item)) => {
                                if let Some(err) = ctx.get_errors().get(0) {
                                    ctx.clear_errors();
                                    return Err(err.clone());
                                }
                                if ctx.ctrl_c.load(Ordering::SeqCst) {
                                    break;
                                }
                            }
                            Ok(None) => {
                                if let Some(err) = ctx.get_errors().get(0) {
                                    ctx.clear_errors();
                                    return Err(err.clone());
                                }
                                break;
                            }
                            Err(e) => return Err(e),
                        }
                    }
                }
            }
            Err(e) => {
                return Err(e);
            }
        }
        output = Ok(InputStream::empty());
        for pipeline in &group.pipelines {
            match output {
                Ok(inp) if inp.is_empty() => {}
                Ok(inp) => {
                    let mut output_stream = inp.to_output_stream();

                    loop {
                        match output_stream.try_next().await {
                            Ok(Some(ReturnSuccess::Value(Value {
                                value: UntaggedValue::Error(e),
                                ..
                            }))) => return Err(e),
                            Ok(Some(_item)) => {
                                if let Some(err) = ctx.get_errors().get(0) {
                                    ctx.clear_errors();
                                    return Err(err.clone());
                                }
                                if ctx.ctrl_c.load(Ordering::SeqCst) {
                                    break;
                                }
                            }
                            Ok(None) => {
                                if let Some(err) = ctx.get_errors().get(0) {
                                    ctx.clear_errors();
                                    return Err(err.clone());
                                }
                                break;
                            }
                            Err(e) => return Err(e),
                        }
                    }
                }
                Err(e) => {
                    return Err(e);
                }
            }
            output = run_pipeline(pipeline, ctx, input).await;

            input = InputStream::empty();
        }
    }

    output
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

            ClassifiedCommand::Internal(left) => run_internal_command(left, ctx, input).await?,
        };
    }

    Ok(input)
}
