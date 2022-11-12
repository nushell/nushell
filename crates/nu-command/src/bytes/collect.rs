use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, SyntaxShape,
    Type, Value,
};

#[derive(Clone, Copy)]
pub struct BytesCollect;

impl Command for BytesCollect {
    fn name(&self) -> &str {
        "bytes collect"
    }

    fn signature(&self) -> Signature {
        Signature::build("bytes collect")
            .input_output_types(vec![(Type::List(Box::new(Type::Binary)), Type::Binary)])
            .optional(
                "separator",
                SyntaxShape::Binary,
                "optional separator to use when creating binary",
            )
            .category(Category::Bytes)
    }

    fn usage(&self) -> &str {
        "Concatenate multiple binary into a single binary, with an optional separator between each"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["join", "concatenate"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let separator: Option<Vec<u8>> = call.opt(engine_state, stack, 0)?;
        // input should be a list of binary data.
        let mut output_binary = vec![];
        for value in input {
            match value {
                Value::Binary { mut val, .. } => {
                    output_binary.append(&mut val);
                    // manually concat
                    // TODO: make use of std::slice::Join when it's available in stable.
                    if let Some(sep) = &separator {
                        let mut work_sep = sep.clone();
                        output_binary.append(&mut work_sep)
                    }
                }
                other => {
                    return Err(ShellError::UnsupportedInput(
                        format!(
                            "The element type is {}, this command only works with bytes.",
                            other.get_type()
                        ),
                        other.span().unwrap_or(call.head),
                    ))
                }
            }
        }

        match separator {
            None => Ok(Value::Binary {
                val: output_binary,
                span: call.head,
            }
            .into_pipeline_data()),
            Some(sep) => {
                if output_binary.is_empty() {
                    Ok(Value::Binary {
                        val: output_binary,
                        span: call.head,
                    }
                    .into_pipeline_data())
                } else {
                    // have push one extra separator in previous step, pop them out.
                    for _ in sep {
                        let _ = output_binary.pop();
                    }
                    Ok(Value::Binary {
                        val: output_binary,
                        span: call.head,
                    }
                    .into_pipeline_data())
                }
            }
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Create a byte array from input",
                example: "[0x[11] 0x[13 15]] | bytes collect",
                result: Some(Value::Binary {
                    val: vec![0x11, 0x13, 0x15],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Create a byte array from input with a separator",
                example: "[0x[11] 0x[33] 0x[44]] | bytes collect 0x[01]",
                result: Some(Value::Binary {
                    val: vec![0x11, 0x01, 0x33, 0x01, 0x44],
                    span: Span::test_data(),
                }),
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(BytesCollect {})
    }
}
