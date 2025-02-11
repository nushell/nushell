use nu_engine::command_prelude::*;

use nu_protocol::{report_parse_warning, ParseWarning};

#[derive(Clone)]
pub struct BitsInto;

impl Command for BitsInto {
    fn name(&self) -> &str {
        "into bits"
    }

    fn signature(&self) -> Signature {
        Signature::build("into bits")
            .input_output_types(vec![
                (Type::Binary, Type::String),
                (Type::Int, Type::String),
                (Type::Filesize, Type::String),
                (Type::Duration, Type::String),
                (Type::String, Type::String),
                (Type::Bool, Type::String),
                (Type::table(), Type::table()),
                (Type::record(), Type::record()),
            ])
            .allow_variants_without_examples(true) // TODO: supply exhaustive examples
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, convert data at the given cell paths.",
            )
            .category(Category::Deprecated)
    }

    fn description(&self) -> &str {
        "Convert value to a binary string."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec![]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        report_parse_warning(
            &StateWorkingSet::new(engine_state),
            &ParseWarning::DeprecatedWarning {
                old_command: "into bits".into(),
                new_suggestion: "use `format bits`".into(),
                span: head,
                url: "`help format bits`".into(),
            },
        );
        crate::extra::strings::format::format_bits(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "convert a binary value into a string, padded to 8 places with 0s",
                example: "0x[1] | into bits",
                result: Some(Value::string("00000001",
                    Span::test_data(),
                )),
            },
            Example {
                description: "convert an int into a string, padded to 8 places with 0s",
                example: "1 | into bits",
                result: Some(Value::string("00000001",
                    Span::test_data(),
                )),
            },
            Example {
                description: "convert a filesize value into a string, padded to 8 places with 0s",
                example: "1b | into bits",
                result: Some(Value::string("00000001",
                    Span::test_data(),
                )),
            },
            Example {
                description: "convert a duration value into a string, padded to 8 places with 0s",
                example: "1ns | into bits",
                result: Some(Value::string("00000001",
                    Span::test_data(),
                )),
            },
            Example {
                description: "convert a boolean value into a string, padded to 8 places with 0s",
                example: "true | into bits",
                result: Some(Value::string("00000001",
                    Span::test_data(),
                )),
            },
            Example {
                description: "convert a string into a raw binary string, padded with 0s to 8 places",
                example: "'nushell.sh' | into bits",
                result: Some(Value::string("01101110 01110101 01110011 01101000 01100101 01101100 01101100 00101110 01110011 01101000",
                    Span::test_data(),
                )),
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

        test_examples(BitsInto {})
    }
}
