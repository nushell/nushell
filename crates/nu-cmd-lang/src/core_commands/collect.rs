use nu_engine::{command_prelude::*, get_eval_block};
use nu_protocol::{engine::CommandType, DataSource, PipelineMetadata};

#[derive(Clone)]
pub struct Collect;

impl Command for Collect {
    fn name(&self) -> &str {
        "collect"
    }

    fn signature(&self) -> Signature {
        Signature::build("collect")
            .input_output_types(vec![(Type::Any, Type::Any)])
            .optional(
                "block",
                SyntaxShape::Block,
                "The block to run once the stream is collected.",
            )
            .category(Category::Core)
    }

    fn command_type(&self) -> CommandType {
        CommandType::Keyword
    }

    fn usage(&self) -> &str {
        "Collect a stream into a value."
    }

    fn extra_usage(&self) -> &str {
        r#"If provided, run a block with the collected value as input.

The entire stream will be collected into one value in memory, so if the stream
is particularly large, this can cause high memory usage."#
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // This is compiled specially by the IR compiler. The code here is never used when
        // running in IR mode.
        let call = call.assert_ast_call()?;
        let block_id = call
            .positional_nth(0)
            .map(|expr| expr.as_block().expect("checked through parser"));

        let metadata = match input.metadata() {
            // Remove the `FilePath` metadata, because after `collect` it's no longer necessary to
            // check where some input came from.
            Some(PipelineMetadata {
                data_source: DataSource::FilePath(_),
                content_type: None,
            }) => None,
            other => other,
        };

        let input = input.into_value(call.head)?;
        let result;

        if let Some(block_id) = block_id {
            let block = engine_state.get_block(block_id);
            let eval_block = get_eval_block(engine_state);

            if let Some(var_id) = block.signature.get_positional(0).and_then(|var| var.var_id) {
                stack.add_var(var_id, input);
                result = eval_block(engine_state, stack, block, PipelineData::Empty);
                stack.remove_var(var_id);
            } else {
                result = eval_block(
                    engine_state,
                    stack,
                    block,
                    input.into_pipeline_data_with_metadata(metadata),
                );
            }
        } else {
            result = Ok(input.into_pipeline_data_with_metadata(metadata));
        }

        result
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Use the second value in the stream",
                example: "[1 2 3] | collect { |x| $x.1 }",
                result: Some(Value::test_int(2)),
            },
            Example {
                description: "Read and write to the same file",
                example: "open file.txt | collect | save -f file.txt",
                result: None,
            },
        ]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Collect {})
    }
}
