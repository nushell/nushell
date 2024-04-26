use super::base64::{operate, ActionType, CHARACTER_SET_DESC};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct EncodeBase64;

impl Command for EncodeBase64 {
    fn name(&self) -> &str {
        "encode base64"
    }

    fn signature(&self) -> Signature {
        Signature::build("encode base64")
            .input_output_types(vec![
                (Type::String, Type::String),
                (Type::Binary, Type::String),
                (
                    Type::List(Box::new(Type::String)),
                    Type::List(Box::new(Type::String)),
                ),
                (
                    Type::List(Box::new(Type::Binary)),
                    Type::List(Box::new(Type::String)),
                ),
                // Relaxed for heterogeneous list.
                // Should be removed as soon as the type system supports better restrictions
                (
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::String)),
                ),
                (Type::table(), Type::table()),
                (Type::record(), Type::record()),
            ])
            .allow_variants_without_examples(true)
            .named(
                "character-set",
                SyntaxShape::String,
                CHARACTER_SET_DESC,
                Some('c'),
            )
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, encode data at the given cell paths.",
            )
            .category(Category::Hash)
    }

    fn usage(&self) -> &str {
        "Encode a string or binary value using Base64."
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Encode binary data",
                example: "0x[09 F9 11 02 9D 74 E3 5B D8 41 56 C5 63 56 88 C0] | encode base64",
                result: Some(Value::test_string("CfkRAp1041vYQVbFY1aIwA==")),
            },
            Example {
                description: "Encode a string with default settings",
                example: "'Some Data' | encode base64",
                result: Some(Value::test_string("U29tZSBEYXRh")),
            },
            Example {
                description: "Encode a string with the binhex character set",
                example: "'Some Data' | encode base64 --character-set binhex",
                result: Some(Value::test_string(r#"7epXB5"%A@4J"#)),
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
