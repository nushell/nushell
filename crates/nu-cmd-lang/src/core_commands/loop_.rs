use nu_engine::{command_prelude::*, get_eval_block};
use nu_protocol::engine::CommandType;

#[derive(Clone)]
pub struct Loop;

impl Command for Loop {
    fn name(&self) -> &str {
        "loop"
    }

    fn description(&self) -> &str {
        "Run a block in a loop."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("loop")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .allow_variants_without_examples(true)
            .required("block", SyntaxShape::Block, "Block to loop.")
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
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // This is compiled specially by the IR compiler. The code here is never used when
        // running in IR mode.
        let call = call.assert_ast_call()?;
        let head = call.head;
        let block_id = call
            .positional_nth(0)
            .expect("checked through parser")
            .as_block()
            .expect("internal error: missing block");

        let block = engine_state.get_block(block_id);
        let eval_block = get_eval_block(engine_state);

        let stack = &mut stack.push_redirection(None, None);

        loop {
            engine_state.signals().check(head)?;

            match eval_block(engine_state, stack, block, PipelineData::empty()) {
                Err(ShellError::Break { .. }) => break,
                Err(ShellError::Continue { .. }) => continue,
                Err(err) => return Err(err),
                Ok(data) => data.drain()?,
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
