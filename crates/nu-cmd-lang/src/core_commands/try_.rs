use nu_engine::{command_prelude::*, get_eval_block, EvalBlockFn};
use nu_protocol::engine::{Closure, CommandType};

#[derive(Clone)]
pub struct Try;

impl Command for Try {
    fn name(&self) -> &str {
        "try"
    }

    fn description(&self) -> &str {
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

    fn extra_description(&self) -> &str {
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
        let head = call.head;
        // This is compiled specially by the IR compiler. The code here is never used when
        // running in IR mode.
        let call = call.assert_ast_call()?;
        let try_block = call
            .positional_nth(0)
            .expect("checked through parser")
            .as_block()
            .expect("internal error: missing block");

        let catch_block: Option<Closure> = call.opt(engine_state, stack, 1)?;

        let try_block = engine_state.get_block(try_block);
        let eval_block = get_eval_block(engine_state);

        let result = eval_block(engine_state, stack, try_block, input)
            .and_then(|pipeline| pipeline.drain_to_out_dests(engine_state, stack));

        match result {
            Err(err) => run_catch(err, head, catch_block, engine_state, stack, eval_block),
            Ok(PipelineData::Value(Value::Error { error, .. }, ..)) => {
                run_catch(*error, head, catch_block, engine_state, stack, eval_block)
            }
            Ok(pipeline) => Ok(pipeline),
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Try to run a division by zero",
                example: "try { 1 / 0 }",
                result: None,
            },
            Example {
                description: "Try to run a division by zero and return a string instead",
                example: "try { 1 / 0 } catch { 'divided by zero' }",
                result: Some(Value::test_string("divided by zero")),
            },
            Example {
                description: "Try to run a division by zero and report the message",
                example: "try { 1 / 0 } catch { |err| $err.msg }",
                result: None,
            },
        ]
    }
}

fn run_catch(
    error: ShellError,
    span: Span,
    catch: Option<Closure>,
    engine_state: &EngineState,
    stack: &mut Stack,
    eval_block: EvalBlockFn,
) -> Result<PipelineData, ShellError> {
    let error = intercept_block_control(error)?;

    if let Some(catch) = catch {
        stack.set_last_error(&error);
        let error = error.into_value(&StateWorkingSet::new(engine_state), span);
        let block = engine_state.get_block(catch.block_id);
        // Put the error value in the positional closure var
        if let Some(var) = block.signature.get_positional(0) {
            if let Some(var_id) = &var.var_id {
                stack.add_var(*var_id, error.clone());
            }
        }

        eval_block(
            engine_state,
            stack,
            block,
            // Make the error accessible with $in, too
            error.into_pipeline_data(),
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
        ShellError::Break { .. } => Err(error),
        ShellError::Continue { .. } => Err(error),
        ShellError::Return { .. } => Err(error),
        _ => Ok(error),
    }
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
