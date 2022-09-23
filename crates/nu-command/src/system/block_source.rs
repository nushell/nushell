use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{CaptureBlock, Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, Signature, Span, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct BlockSource;

impl Command for BlockSource {
    fn name(&self) -> &str {
        "bs"
    }

    fn signature(&self) -> Signature {
        Signature::build("bs")
            .required(
                "block",
                SyntaxShape::BlockWithSource,
                "block with source code",
            )
            .category(Category::System)
    }

    fn usage(&self) -> &str {
        "return source code from given block"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let block: CaptureBlock = call.req(engine_state, stack, 0)?;
        let block_source = &engine_state.get_block(block.block_id).source;
        Ok(match block_source {
            None => Value::Nothing { span: call.head },
            Some(code) => Value::String {
                val: code.clone(),
                span: call.head,
            },
        }
        .into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Get source code from block { echo $x }",
            example: " bs { echo 300 } ",
            result: Some(Value::String {
                val: " echo 300 ".to_string(),
                span: Span::test_data(),
            }),
        }]
    }
}

#[cfg(test)]
mod test {
    use crate::BlockSource;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(BlockSource {})
    }
}
