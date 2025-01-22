use nu_engine::command_prelude::*;
use nu_protocol::format_filesize_from_conf;
use rand::{thread_rng, RngCore};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
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
                    actual: format_filesize_from_conf(val, engine_state.get_config()),
                    span: length_val.span(),
                })
            }
            val => Err(ShellError::RuntimeTypeMismatch {
                expected: Type::custom("int or filesize"),
                actual: val.get_type(),
                span: val.span(),
            }),
        }?;

        let mut rng = thread_rng();

        let mut out = vec![0u8; length];
        rng.fill_bytes(&mut out);

        Ok(Value::binary(out, call.head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
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

        test_examples(SubCommand {})
    }
}
