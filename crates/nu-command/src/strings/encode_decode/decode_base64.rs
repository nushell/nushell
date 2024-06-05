use super::base64::{operate, ActionType, Base64CommandArguments, CHARACTER_SET_DESC};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct DecodeBase64;

impl Command for DecodeBase64 {
    fn name(&self) -> &str {
        "decode base64"
    }

    fn signature(&self) -> Signature {
        Signature::build("decode base64")
            .input_output_types(vec![
                (Type::String, Type::Any),
                (
                    Type::List(Box::new(Type::String)),
                    Type::List(Box::new(Type::Any)),
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
            .switch(
                "binary",
                "Output a binary value instead of decoding payload as UTF-8",
                Some('b'),
            )
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, decode data at the given cell paths.",
            )
            .category(Category::Hash)
    }

    fn usage(&self) -> &str {
        "Base64 decode a value."
    }

    fn extra_usage(&self) -> &str {
        r#"Will attempt to decode binary payload as an UTF-8 string by default. Use the `--binary(-b)` argument to force binary output."#
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Base64 decode a value and output as UTF-8 string",
                example: "'U29tZSBEYXRh' | decode base64",
                result: Some(Value::test_string("Some Data")),
            },
            Example {
                description: "Base64 decode a value and output as binary",
                example: "'U29tZSBEYXRh' | decode base64 --binary",
                result: Some(Value::binary(
                    [0x53, 0x6f, 0x6d, 0x65, 0x20, 0x44, 0x61, 0x74, 0x61],
                    Span::test_data(),
                )),
            },
        ]
    }

    fn is_const(&self) -> bool {
        true
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let character_set: Option<Spanned<String>> =
            call.get_flag(engine_state, stack, "character-set")?;
        let binary = call.has_flag(engine_state, stack, "binary")?;
        let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
        let args = Base64CommandArguments {
            action_type: ActionType::Decode,
            binary,
            character_set,
        };
        operate(engine_state, call, input, cell_paths, args)
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let character_set: Option<Spanned<String>> =
            call.get_flag_const(working_set, "character-set")?;
        let binary = call.has_flag_const(working_set, "binary")?;
        let cell_paths: Vec<CellPath> = call.rest_const(working_set, 0)?;
        let args = Base64CommandArguments {
            action_type: ActionType::Decode,
            binary,
            character_set,
        };
        operate(working_set.permanent(), call, input, cell_paths, args)
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
