use nu_engine::{command_prelude::*, get_eval_block, EvalBlockFn};
use nu_protocol::engine::{Closure, CommandType};

#[derive(Clone)]
pub struct Try;

impl Command for Try {
    fn name(&self) -> &str {
        "try"
    }

    fn usage(&self) -> &str {
        "Try to run a block, if it fails optionally run a catch closure."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("try")
            .input_output_types(vec![(Type::Any, Type::Any)])
            .required("try_block", SyntaxShape::Block, "Block to run.")
            .optional(
                "catch_closure",
                SyntaxShape::Keyword(
                    b"catch".to_vec(),
                    Box::new(SyntaxShape::OneOf(vec![
                        SyntaxShape::Closure(None),
                        SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                    ])),
                ),
                "Closure to run if try block fails.",
            )
            .category(Category::Core)
    }

    fn extra_usage(&self) -> &str {
        r#"This command is a parser keyword. For details, check:
  https://www.nushell.sh/book/thinking_in_nu.html"#
    }

    fn command_type(&self) -> CommandType {
        CommandType::Keyword
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let try_block = call
            .positional_nth(0)
            .expect("checked through parser")
            .as_block()
            .expect("internal error: missing block");

        let catch_block: Option<Closure> = call.opt(engine_state, stack, 1)?;

        let try_block = engine_state.get_block(try_block);
        let eval_block = get_eval_block(engine_state);

        match eval_block(engine_state, stack, try_block, input) {
            Err(error) => {
                let error = intercept_block_control(error)?;
                let err_record = err_to_record(error, call.head);
                handle_catch(err_record, catch_block, engine_state, stack, eval_block)
            }
            Ok(PipelineData::Value(Value::Error { error, .. }, ..)) => {
                let error = intercept_block_control(*error)?;
                let err_record = err_to_record(error, call.head);
                handle_catch(err_record, catch_block, engine_state, stack, eval_block)
            }
            // external command may fail to run
            Ok(pipeline) => {
                let (pipeline, external_failed) = pipeline.check_external_failed()?;
                if external_failed {
                    let status = pipeline.drain()?;
                    let code = status.map(|status| status.code()).unwrap_or(0);
                    stack.add_env_var("LAST_EXIT_CODE".into(), Value::int(code.into(), call.head));
                    let err_value = Value::nothing(call.head);
                    handle_catch(err_value, catch_block, engine_state, stack, eval_block)
                } else {
                    Ok(pipeline)
                }
            }
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Try to run a missing command",
                example: "try { asdfasdf }",
                result: None,
            },
            Example {
                description: "Try to run a missing command",
                example: "try { asdfasdf } catch { 'missing' }",
                result: Some(Value::test_string("missing")),
            },
            Example {
                description: "Try to run a missing command and report the message",
                example: "try { asdfasdf } catch { |err| $err.msg }",
                result: None,
            },
        ]
    }
}

fn handle_catch(
    err_value: Value,
    catch_block: Option<Closure>,
    engine_state: &EngineState,
    stack: &mut Stack,
    eval_block_fn: EvalBlockFn,
) -> Result<PipelineData, ShellError> {
    if let Some(catch_block) = catch_block {
        let catch_block = engine_state.get_block(catch_block.block_id);
        // Put the error value in the positional closure var
        if let Some(var) = catch_block.signature.get_positional(0) {
            if let Some(var_id) = &var.var_id {
                stack.add_var(*var_id, err_value.clone());
            }
        }

        eval_block_fn(
            engine_state,
            stack,
            catch_block,
            // Make the error accessible with $in, too
            err_value.into_pipeline_data(),
        )
    } else {
        Ok(PipelineData::empty())
    }
}

/// The flow control commands `break`/`continue`/`return` emit their own [`ShellError`] variants
/// We need to ignore those in `try` and bubble them through
///
/// `Err` when flow control to bubble up with `?`
fn intercept_block_control(error: ShellError) -> Result<ShellError, ShellError> {
    match error {
        nu_protocol::ShellError::Break { .. } => Err(error),
        nu_protocol::ShellError::Continue { .. } => Err(error),
        nu_protocol::ShellError::Return { .. } => Err(error),
        _ => Ok(error),
    }
}

/// Convert from `error` to [`Value::Record`] so the error information can be easily accessed in catch.
fn err_to_record(error: ShellError, head: Span) -> Value {
    Value::record(
        record! {
            "msg" => Value::string(error.to_string(), head),
            "debug" => Value::string(format!("{error:?}"), head),
            "raw" => Value::error(error, head),
        },
        head,
    )
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Try {})
    }
}
