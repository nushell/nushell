use crate::commands::classified::expr::run_expression_block;
use crate::commands::classified::internal::run_internal_command;
use crate::context::Context;
use crate::prelude::*;
use crate::stream::InputStream;
use futures::stream::TryStreamExt;
use nu_errors::ShellError;
use nu_protocol::hir::{Block, ClassifiedCommand, Commands};
use nu_protocol::{ReturnSuccess, Scope, UntaggedValue, Value};
use std::sync::atomic::Ordering;

pub(crate) async fn run_block(
    block: &Block,
    ctx: &mut Context,
    mut input: InputStream,
    scope: &Scope,
) -> Result<InputStream, ShellError> {
    let mut output: Result<InputStream, ShellError> = Ok(InputStream::empty());
    for pipeline in &block.block {
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
        output = run_pipeline(pipeline, ctx, input, scope).await;

        input = InputStream::empty();
    }

    output
}

async fn run_pipeline(
    commands: &Commands,
    ctx: &mut Context,
    mut input: InputStream,
    scope: &Scope,
) -> Result<InputStream, ShellError> {
    let mut iter = commands.list.clone().into_iter().peekable();

    loop {
        let item: Option<ClassifiedCommand> = iter.next();
        let next: Option<&ClassifiedCommand> = iter.peek();

        input = match (item, next) {
            (Some(ClassifiedCommand::Dynamic(_)), _) | (_, Some(ClassifiedCommand::Dynamic(_))) => {
                return Err(ShellError::unimplemented("Dynamic commands"))
            }

            (Some(ClassifiedCommand::Expr(expr)), _) => {
                run_expression_block(*expr, ctx, input, scope)?
            }
            (Some(ClassifiedCommand::Error(err)), _) => return Err(err.into()),
            (_, Some(ClassifiedCommand::Error(err))) => return Err(err.clone().into()),

            (Some(ClassifiedCommand::Internal(left)), _) => {
                run_internal_command(left, ctx, input, scope)?
            }

            (None, _) => break,
        };
    }

    Ok(input)
}
