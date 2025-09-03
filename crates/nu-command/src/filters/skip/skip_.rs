use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct Skip;

impl Command for Skip {
    fn name(&self) -> &str {
        "skip"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![
                (Type::table(), Type::table()),
                (Type::Binary, Type::Binary),
                (
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::Any)),
                ),
            ])
            .optional("n", SyntaxShape::Int, "The number of elements to skip.")
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Skip the first several rows of the input. Counterpart of `drop`. Opposite of `first`."
    }

    fn extra_description(&self) -> &str {
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
                result: Some(Value::test_list(vec![
                    Value::test_int(4),
                    Value::test_int(6),
                    Value::test_int(8),
                ])),
            },
            Example {
                description: "Skip two rows of a table",
                example: "[[editions]; [2015] [2018] [2021]] | skip 2",
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "editions" => Value::test_int(2021),
                })])),
            },
            Example {
                description: "Skip 2 bytes of a binary value",
                example: "0x[01 23 45 67] | skip 2",
                result: Some(Value::test_binary(vec![0x45, 0x67])),
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
        let metadata = input.metadata().map(|m| m.with_content_type(None));

        let n: usize = match n {
            Some(v) => {
                let span = v.span();
                match v {
                    Value::Int { val, .. } => {
                        val.try_into().map_err(|err| ShellError::TypeMismatch {
                            err_message: format!("Could not convert {val} to unsigned int: {err}"),
                            span,
                        })?
                    }
                    _ => {
                        return Err(ShellError::TypeMismatch {
                            err_message: "expected int".into(),
                            span,
                        });
                    }
                }
            }
            None => 1,
        };
        let input_span = input.span().unwrap_or(call.head);
        match input {
            PipelineData::ByteStream(stream, metadata) => {
                if stream.type_().is_binary_coercible() {
                    let span = stream.span();
                    Ok(PipelineData::byte_stream(
                        stream.skip(span, n as u64)?,
                        // last 5 bytes of an image/png stream are not image/png themselves
                        metadata.map(|m| m.with_content_type(None)),
                    ))
                } else {
                    Err(ShellError::OnlySupportsThisInputType {
                        exp_input_type: "list, binary or range".into(),
                        wrong_type: stream.type_().describe().into(),
                        dst_span: call.head,
                        src_span: stream.span(),
                    })
                }
            }
            PipelineData::Value(Value::Binary { val, .. }, metadata) => {
                let bytes = val.into_iter().skip(n).collect::<Vec<_>>();
                let metadata = metadata.map(|m| m.with_content_type(None));
                Ok(Value::binary(bytes, input_span).into_pipeline_data_with_metadata(metadata))
            }
            _ => Ok(input
                .into_iter_strict(call.head)?
                .skip(n)
                .into_pipeline_data_with_metadata(
                    input_span,
                    engine_state.signals().clone(),
                    metadata,
                )),
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
