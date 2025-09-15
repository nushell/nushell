use super::hex::{ActionType, operate};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct DecodeHex;

impl Command for DecodeHex {
    fn name(&self) -> &str {
        "decode hex"
    }

    fn signature(&self) -> Signature {
        Signature::build("decode hex")
            .input_output_types(vec![
                (Type::String, Type::Binary),
                (
                    Type::List(Box::new(Type::String)),
                    Type::List(Box::new(Type::Binary)),
                ),
                (Type::table(), Type::table()),
                (Type::record(), Type::record()),
            ])
            .allow_variants_without_examples(true)
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, decode data at the given cell paths",
            )
            .category(Category::Formats)
    }

    fn description(&self) -> &str {
        "Hex decode a value."
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Hex decode a value and output as binary",
                example: "'0102030A0a0B' | decode hex",
                result: Some(Value::binary(
                    [0x01, 0x02, 0x03, 0x0A, 0x0A, 0x0B],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Whitespaces are allowed to be between hex digits",
                example: "'01 02  03 0A 0a 0B' | decode hex",
                result: Some(Value::binary(
                    [0x01, 0x02, 0x03, 0x0A, 0x0A, 0x0B],
                    Span::test_data(),
                )),
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
        operate(ActionType::Decode, engine_state, stack, call, input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        crate::test_examples(DecodeHex)
    }
}
