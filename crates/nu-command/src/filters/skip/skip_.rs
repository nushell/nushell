use std::convert::TryInto;

use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData, ShellError,
    Signature, Span, SpannedValue, SyntaxShape, Type,
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
        "Skip the first several rows of the input. Counterpart of `drop`. Opposite of `first`."
    }

    fn extra_usage(&self) -> &str {
        r#"To skip specific numbered rows, try `drop nth`. To skip specific named columns, try `reject`."#
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["ignore", "remove", "last", "slice", "tail"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Skip the first value of a list",
                example: "[2 4 6 8] | skip 1",
                result: Some(SpannedValue::List {
                    vals: vec![
                        SpannedValue::test_int(4),
                        SpannedValue::test_int(6),
                        SpannedValue::test_int(8),
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Skip two rows of a table",
                example: "[[editions]; [2015] [2018] [2021]] | skip 2",
                result: Some(SpannedValue::List {
                    vals: vec![SpannedValue::Record {
                        cols: vec!["editions".to_owned()],
                        vals: vec![SpannedValue::test_int(2021)],
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
        let n: Option<SpannedValue> = call.opt(engine_state, stack, 0)?;
        let span = call.head;
        let metadata = input.metadata();

        let n: usize = match n {
            Some(SpannedValue::Int { val, span }) => {
                val.try_into().map_err(|err| ShellError::TypeMismatch {
                    err_message: format!("Could not convert {val} to unsigned integer: {err}"),
                    span,
                })?
            }
            Some(_) => {
                return Err(ShellError::TypeMismatch {
                    err_message: "expected integer".into(),
                    span,
                })
            }
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
                        SpannedValue::String { val, .. } => {
                            let bytes = val.as_bytes();
                            if bytes.len() < remaining {
                                remaining -= bytes.len();
                                //output.extend_from_slice(bytes)
                            } else {
                                output.extend_from_slice(&bytes[remaining..]);
                                break;
                            }
                        }
                        SpannedValue::Binary { val: bytes, .. } => {
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

                Ok(SpannedValue::Binary {
                    val: output,
                    span: bytes_span,
                }
                .into_pipeline_data()
                .set_metadata(metadata))
            }
            PipelineData::Value(SpannedValue::Binary { val, span }, metadata) => {
                let bytes = val.into_iter().skip(n).collect::<Vec<_>>();

                Ok(SpannedValue::Binary { val: bytes, span }
                    .into_pipeline_data()
                    .set_metadata(metadata))
            }
            _ => Ok(input
                .into_iter_strict(call.head)?
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
