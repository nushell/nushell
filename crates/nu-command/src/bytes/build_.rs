use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct BytesBuild;

impl Command for BytesBuild {
    fn name(&self) -> &str {
        "bytes build"
    }

    fn description(&self) -> &str {
        "Create bytes from the arguments."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["concatenate", "join"]
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("bytes build")
            .input_output_types(vec![(Type::Nothing, Type::Binary)])
            .rest("rest", SyntaxShape::Any, "List of bytes.")
            .category(Category::Bytes)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: "bytes build 0x[01 02] 0x[03] 0x[04]",
                description: "Builds binary data from 0x[01 02], 0x[03], 0x[04]",
                result: Some(Value::binary(
                    vec![0x01, 0x02, 0x03, 0x04],
                    Span::test_data(),
                )),
            },
            Example {
                example: "bytes build 255 254 253 252",
                description: "Builds binary data from byte numbers",
                result: Some(Value::test_binary(vec![0xff, 0xfe, 0xfd, 0xfc])),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let mut output = vec![];
        for val in call.rest::<Value>(engine_state, stack, 0)? {
            let val_span = val.span();
            match val {
                Value::Binary { mut val, .. } => output.append(&mut val),
                Value::Int { val, .. } => {
                    let byte: u8 = val.try_into().map_err(|_| ShellError::IncorrectValue {
                        msg: format!("{val} is out of range for byte"),
                        val_span,
                        call_span: call.head,
                    })?;
                    output.push(byte);
                }
                // Explicitly propagate errors instead of dropping them.
                Value::Error { error, .. } => return Err(*error),
                other => {
                    return Err(ShellError::TypeMismatch {
                        err_message: "only binary data arguments are supported".to_string(),
                        span: other.span(),
                    });
                }
            }
        }

        Ok(Value::binary(output, call.head).into_pipeline_data())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(BytesBuild {})
    }
}
