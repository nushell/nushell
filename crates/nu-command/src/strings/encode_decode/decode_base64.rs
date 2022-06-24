use super::base64::{operate, ActionType, CHARACTER_SET_DESC};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct DecodeBase64;

impl Command for DecodeBase64 {
    fn name(&self) -> &str {
        "decode base64"
    }

    fn signature(&self) -> Signature {
        Signature::build("decode base64")
            .named(
                "character-set",
                SyntaxShape::String,
                CHARACTER_SET_DESC,
                Some('c'),
            )
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "optionally base64 decode data by column paths",
            )
            .category(Category::Hash)
    }

    fn usage(&self) -> &str {
        "base64 decode a value"
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Base64 decode a value",
            example: "echo 'U29tZSBEYXRh' | decode base64",
            result: Some(Value::binary(
                [0x53, 0x6f, 0x6d, 0x65, 0x20, 0x44, 0x61, 0x74, 0x61],
                Span::test_data(),
            )),
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        operate(ActionType::Decode, engine_state, stack, call, input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        crate::test_examples(DecodeBase64)
    }
}
