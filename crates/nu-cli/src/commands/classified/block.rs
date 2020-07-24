use crate::commands::classified::expr::run_expression_block;
use crate::commands::classified::internal::run_internal_command;
use crate::context::Context;
use crate::prelude::*;
use crate::stream::InputStream;
use futures::stream::TryStreamExt;
use nu_errors::ShellError;
use nu_protocol::hir::{Block, ClassifiedCommand, Commands};
use nu_protocol::{ReturnSuccess, UntaggedValue, Value};
use std::sync::atomic::Ordering;

pub(crate) async fn run_block(
    block: &Block,
    ctx: &mut Context,
    mut input: InputStream,
    it: &Value,
    vars: &IndexMap<String, Value>,
    env: &IndexMap<String, String>,
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
        output = run_pipeline(pipeline, ctx, input, it, vars, env).await;

        input = InputStream::empty();
    }

    output
}

async fn run_pipeline(
    commands: &Commands,
    ctx: &mut Context,
    mut input: InputStream,
    it: &Value,
    vars: &IndexMap<String, Value>,
    env: &IndexMap<String, String>,
) -> Result<InputStream, ShellError> {
    for item in commands.list.clone() {
        input = match item {
            ClassifiedCommand::Dynamic(_) => {
                return Err(ShellError::unimplemented("Dynamic commands"))
            }

            ClassifiedCommand::Expr(expr) => {
                run_expression_block(*expr, ctx, it, vars, env).await?
            }

            ClassifiedCommand::Error(err) => return Err(err.into()),

            ClassifiedCommand::Internal(left) => {
                run_internal_command(left, ctx, input, it, vars, env).await?
            }
        };
    }

    Ok(input)
}
