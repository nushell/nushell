use nu_engine::{eval_block, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Block, Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
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
            .required("block", SyntaxShape::Block, "block to loop")
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
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
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
                Err(ShellError::Break(_)) => {
                    break;
                }
                Err(ShellError::Continue(_)) => {
                    continue;
                }
                Err(err) => {
                    return Err(err);
                }
                Ok(pipeline) => {
                    let exit_code = pipeline.print(engine_state, stack, false, false)?;
                    if exit_code != 0 {
                        break;
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
            result: Some(Value::int(11, Span::test_data())),
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
