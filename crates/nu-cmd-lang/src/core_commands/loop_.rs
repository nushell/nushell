use nu_engine::{eval_block, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Block, Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct Loop;

impl Command for Loop {
    fn name(&self) -> &str {
        "loop"
    }

    fn usage(&self) -> &str {
        "Run a block in a loop."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("loop")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .allow_variants_without_examples(true)
            .required("block", SyntaxShape::Block, "Block to loop.")
            .category(Category::Core)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let block: Block = call.req(engine_state, stack, 0)?;

        loop {
            if nu_utils::ctrl_c::was_pressed(&engine_state.ctrlc) {
                break;
            }

            let block = engine_state.get_block(block.block_id);
            match eval_block(
                engine_state,
                stack,
                block,
                PipelineData::empty(),
                call.redirect_stdout,
                call.redirect_stderr,
            ) {
                Err(ShellError::Break { .. }) => {
                    break;
                }
                Err(ShellError::Continue { .. }) => {
                    continue;
                }
                Err(err) => {
                    return Err(err);
                }
                Ok(pipeline) => {
                    let exit_code = pipeline.drain_with_exit_code()?;
                    if exit_code != 0 {
                        return Ok(PipelineData::new_external_stream_with_only_exit_code(
                            exit_code,
                        ));
                    }
                }
            }
        }
        Ok(PipelineData::empty())
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Loop while a condition is true",
            example: "mut x = 0; loop { if $x > 10 { break }; $x = $x + 1 }; $x",
            result: Some(Value::test_int(11)),
        }]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Loop {})
    }
}
