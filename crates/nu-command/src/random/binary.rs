use super::byte_stream::{RandomDistribution, random_byte_stream};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct RandomBinary;

impl Command for RandomBinary {
    fn name(&self) -> &str {
        "random binary"
    }

    fn signature(&self) -> Signature {
        Signature::build("random binary")
            .input_output_types(vec![(Type::Nothing, Type::Binary)])
            .allow_variants_without_examples(true)
            .required(
                "length",
                SyntaxShape::OneOf(vec![SyntaxShape::Int, SyntaxShape::Filesize]),
                "Length of the output binary.",
            )
            .category(Category::Random)
    }

    fn description(&self) -> &str {
        "Generate random bytes."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["generate", "bytes"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let length_val = call.req(engine_state, stack, 0)?;
        let length = match length_val {
            Value::Int { val, .. } => usize::try_from(val).map_err(|_| ShellError::InvalidValue {
                valid: "a non-negative int or filesize".into(),
                actual: val.to_string(),
                span: length_val.span(),
            }),
            Value::Filesize { val, .. } => {
                usize::try_from(val).map_err(|_| ShellError::InvalidValue {
                    valid: "a non-negative int or filesize".into(),
                    actual: engine_state.get_config().filesize.format(val).to_string(),
                    span: length_val.span(),
                })
            }
            val => Err(ShellError::RuntimeTypeMismatch {
                expected: Type::custom("int or filesize"),
                actual: val.get_type(),
                span: val.span(),
            }),
        }?;

        Ok(random_byte_stream(
            RandomDistribution::Binary,
            length,
            call.head,
            engine_state.signals().clone(),
        ))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Generate 16 random bytes",
                example: "random binary 16",
                result: None,
            },
            Example {
                description: "Generate 1 random kilobyte",
                example: "random binary 1kb",
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

        test_examples(RandomBinary {})
    }
}
