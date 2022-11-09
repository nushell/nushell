use super::base64::{operate, ActionType, CHARACTER_SET_DESC};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct EncodeBase64;

impl Command for EncodeBase64 {
    fn name(&self) -> &str {
        "encode base64"
    }

    fn signature(&self) -> Signature {
        Signature::build("encode base64")
            .input_output_types(vec![(Type::String, Type::String)])
            .vectorizes_over_list(true)
            .named(
                "character-set",
                SyntaxShape::String,
                CHARACTER_SET_DESC,
                Some('c'),
            )
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, encode data at the given cell paths",
            )
            .output_type(Type::String)
            .category(Category::Hash)
    }

    fn usage(&self) -> &str {
        "Base64 encode a value"
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Base64 encode a string with default settings",
                example: "echo 'Some Data' | encode base64",
                result: Some(Value::string("U29tZSBEYXRh", Span::test_data())),
            },
            Example {
                description: "Base64 encode a string with the binhex character set",
                example: "echo 'Some Data' | encode base64 --character-set binhex",
                result: Some(Value::string(r#"7epXB5"%A@4J"#, Span::test_data())),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        operate(ActionType::Encode, engine_state, stack, call, input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        crate::test_examples(EncodeBase64)
    }
}
