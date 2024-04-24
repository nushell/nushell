use nu_engine::{command_prelude::*, get_eval_expression};

#[derive(Clone)]
pub struct BytesBuild;

impl Command for BytesBuild {
    fn name(&self) -> &str {
        "bytes build"
    }

    fn usage(&self) -> &str {
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

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            example: "bytes build 0x[01 02] 0x[03] 0x[04]",
            description: "Builds binary data from 0x[01 02], 0x[03], 0x[04]",
            result: Some(Value::binary(
                vec![0x01, 0x02, 0x03, 0x04],
                Span::test_data(),
            )),
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let mut output = vec![];
        for val in call.rest_iter_flattened(0, |expr| {
            let eval_expression = get_eval_expression(engine_state);
            eval_expression(engine_state, stack, expr)
        })? {
            match val {
                Value::Binary { mut val, .. } => output.append(&mut val),
                // Explicitly propagate errors instead of dropping them.
                Value::Error { error, .. } => return Err(*error),
                other => {
                    return Err(ShellError::TypeMismatch {
                        err_message: "only binary data arguments are supported".to_string(),
                        span: other.span(),
                    })
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
