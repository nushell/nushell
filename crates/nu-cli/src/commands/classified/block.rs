use crate::commands::classified::expr::run_expression_block;
use crate::commands::classified::internal::run_internal_command;
use crate::evaluation_context::EvaluationContext;
use crate::prelude::*;
use crate::stream::InputStream;
use futures::stream::TryStreamExt;
use nu_errors::ShellError;
use nu_protocol::hir::{
    Block, Call, ClassifiedCommand, Expression, Pipeline, SpannedExpression, Synthetic,
};
use nu_protocol::{ReturnSuccess, Scope, UntaggedValue, Value};
use std::sync::atomic::Ordering;

pub(crate) async fn run_block(
    block: &Block,
    ctx: &mut EvaluationContext,
    mut input: InputStream,
    scope: Arc<Scope>,
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
                            scope.clone(),
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
            output = run_pipeline(pipeline, ctx, input, scope.clone()).await;

            input = InputStream::empty();
        }
    }

    output
}

async fn run_pipeline(
    commands: &Pipeline,
    ctx: &mut EvaluationContext,
    mut input: InputStream,
    scope: Arc<Scope>,
) -> Result<InputStream, ShellError> {
    for item in commands.list.clone() {
        input = match item {
            ClassifiedCommand::Dynamic(_) => {
                return Err(ShellError::unimplemented("Dynamic commands"))
            }

            ClassifiedCommand::Expr(expr) => {
                run_expression_block(*expr, ctx, scope.clone()).await?
            }

            ClassifiedCommand::Error(err) => return Err(err.into()),

            ClassifiedCommand::Internal(left) => {
                run_internal_command(left, ctx, input, scope.clone()).await?
            }
        };
    }

    Ok(input)
}
