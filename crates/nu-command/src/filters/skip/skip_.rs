use std::convert::TryInto;

use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData, ShellError,
    Signature, Span, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct Skip;

impl Command for Skip {
    fn name(&self) -> &str {
        "skip"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![
                (Type::Table(vec![]), Type::Table(vec![])),
                (
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::Any)),
                ),
            ])
            .optional("n", SyntaxShape::Int, "the number of elements to skip")
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Skip the first several rows of the input. Counterpart of 'drop'. Opposite of 'first'."
    }

    fn extra_usage(&self) -> &str {
        r#"To skip specific numbered rows, try 'drop nth'. To skip specific named columns, try 'reject'."#
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["ignore", "remove", "last", "slice", "tail"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Skip the first value of a list",
                example: "echo [2 4 6 8] | skip 1",
                result: Some(Value::List {
                    vals: vec![Value::test_int(4), Value::test_int(6), Value::test_int(8)],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Skip two rows of a table",
                example: "echo [[editions]; [2015] [2018] [2021]] | skip 2",
                result: Some(Value::List {
                    vals: vec![Value::Record {
                        cols: vec!["editions".to_owned()],
                        vals: vec![Value::test_int(2021)],
                        span: Span::test_data(),
                    }],
                    span: Span::test_data(),
                }),
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
        let n: Option<Value> = call.opt(engine_state, stack, 0)?;
        let span = call.head;
        let metadata = input.metadata();

        let n: usize = match n {
            Some(Value::Int { val, span }) => val.try_into().map_err(|err| {
                ShellError::UnsupportedInput(
                    format!("Could not convert {} to unsigned integer: {}", val, err),
                    span,
                )
            })?,
            Some(_) => return Err(ShellError::TypeMismatch("expected integer".into(), span)),
            None => 1,
        };

        let ctrlc = engine_state.ctrlc.clone();

        match input {
            PipelineData::ExternalStream {
                stdout: Some(stream),
                span: bytes_span,
                metadata,
                ..
            } => {
                let mut remaining = n;
                let mut output = vec![];

                for frame in stream {
                    let frame = frame?;

                    match frame {
                        Value::String { val, .. } => {
                            let bytes = val.as_bytes();
                            if bytes.len() < remaining {
                                remaining -= bytes.len();
                                //output.extend_from_slice(bytes)
                            } else {
                                output.extend_from_slice(&bytes[remaining..]);
                                break;
                            }
                        }
                        Value::Binary { val: bytes, .. } => {
                            if bytes.len() < remaining {
                                remaining -= bytes.len();
                            } else {
                                output.extend_from_slice(&bytes[remaining..]);
                                break;
                            }
                        }
                        _ => unreachable!("Raw streams are either bytes or strings"),
                    }
                }

                Ok(Value::Binary {
                    val: output,
                    span: bytes_span,
                }
                .into_pipeline_data()
                .set_metadata(metadata))
            }
            PipelineData::Value(Value::Binary { val, span }, metadata) => {
                let bytes = val.into_iter().skip(n).collect::<Vec<_>>();

                Ok(Value::Binary { val: bytes, span }
                    .into_pipeline_data()
                    .set_metadata(metadata))
            }
            _ => Ok(input
                .into_iter()
                .skip(n)
                .into_pipeline_data(ctrlc)
                .set_metadata(metadata)),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Skip;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Skip {})
    }
}
