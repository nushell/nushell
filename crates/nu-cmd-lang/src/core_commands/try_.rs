use nu_engine::{eval_block, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Block, Closure, Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, SyntaxShape,
    Type, Value,
};

#[derive(Clone)]
pub struct Try;

impl Command for Try {
    fn name(&self) -> &str {
        "try"
    }

    fn usage(&self) -> &str {
        "Try to run a block, if it fails optionally run a catch block."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("try")
            .input_output_types(vec![(Type::Any, Type::Any)])
            .required("try_block", SyntaxShape::Block, "block to run")
            .optional(
                "catch_block",
                SyntaxShape::Keyword(
                    b"catch".to_vec(),
                    Box::new(SyntaxShape::Closure(Some(vec![SyntaxShape::Any]))),
                ),
                "block to run if try block fails",
            )
            .category(Category::Core)
    }

    fn extra_usage(&self) -> &str {
        r#"This command is a parser keyword. For details, check:
  https://www.nushell.sh/book/thinking_in_nu.html"#
    }

    fn is_parser_keyword(&self) -> bool {
        true
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let try_block: Block = call.req(engine_state, stack, 0)?;
        let catch_block: Option<Closure> = call.opt(engine_state, stack, 1)?;

        let try_block = engine_state.get_block(try_block.block_id);

        let result = eval_block(engine_state, stack, try_block, input, false, false);

        match result {
            Err(error) => {
                let error = intercept_block_control(error)?;
                let err_record = err_to_record(error, call.head);
                handle_catch(err_record, catch_block, engine_state, stack)
            }
            Ok(PipelineData::Value(Value::Error { error }, ..)) => {
                let error = intercept_block_control(*error)?;
                let err_record = err_to_record(error, call.head);
                handle_catch(err_record, catch_block, engine_state, stack)
            }
            // external command may fail to run
            Ok(pipeline) => {
                let (pipeline, external_failed) = pipeline.is_external_failed();
                if external_failed {
                    // Because external command errors aren't "real" errors,
                    // (unless do -c is in effect)
                    // they can't be passed in as Nushell values.
                    let err_value = Value::nothing(call.head);
                    handle_catch(err_value, catch_block, engine_state, stack)
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
                example: "try { asdfasdf } catch { echo 'missing' } ",
                result: Some(Value::test_string("missing")),
            },
        ]
    }
}

fn handle_catch(
    err_value: Value,
    catch_block: Option<Closure>,
    engine_state: &EngineState,
    stack: &mut Stack,
) -> Result<PipelineData, ShellError> {
    if let Some(catch_block) = catch_block {
        let catch_block = engine_state.get_block(catch_block.block_id);
        // Put the error value in the positional closure var
        if let Some(var) = catch_block.signature.get_positional(0) {
            if let Some(var_id) = &var.var_id {
                stack.add_var(*var_id, err_value.clone());
            }
        }

        eval_block(
            engine_state,
            stack,
            catch_block,
            // Make the error accessible with $in, too
            err_value.into_pipeline_data(),
            false,
            false,
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
        nu_protocol::ShellError::Break(_) => Err(error),
        nu_protocol::ShellError::Continue(_) => Err(error),
        nu_protocol::ShellError::Return(_, _) => Err(error),
        _ => Ok(error),
    }
}

/// Convert from `error` to [`Value::Record`] so the error information can be easily accessed in catch.
fn err_to_record(error: ShellError, head: Span) -> Value {
    let cols = vec!["msg".to_string(), "detail".to_string(), "raw".to_string()];
    let vals = vec![
        Value::string(error.to_string(), head),
        Value::string(format!("{error:?}"), head),
        Value::Error {
            error: Box::new(error),
        },
    ];
    Value::record(cols, vals, head)
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
