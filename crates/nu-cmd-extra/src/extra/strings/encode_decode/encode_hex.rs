use super::hex::{ActionType, operate};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct EncodeHex;

impl Command for EncodeHex {
    fn name(&self) -> &str {
        "encode hex"
    }

    fn signature(&self) -> Signature {
        Signature::build("encode hex")
            .input_output_types(vec![
                (Type::Binary, Type::String),
                (
                    Type::List(Box::new(Type::Binary)),
                    Type::List(Box::new(Type::String)),
                ),
                (Type::table(), Type::table()),
                (Type::record(), Type::record()),
            ])
            .allow_variants_without_examples(true)
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, encode data at the given cell paths",
            )
            .category(Category::Formats)
    }

    fn description(&self) -> &str {
        "Encode a binary value using hex."
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Encode binary data",
            example: "0x[09 F9 11 02 9D 74 E3 5B D8 41 56 C5 63 56 88 C0] | encode hex",
            result: Some(Value::test_string("09F911029D74E35BD84156C5635688C0")),
        }]
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
        crate::test_examples(EncodeHex)
    }
}
