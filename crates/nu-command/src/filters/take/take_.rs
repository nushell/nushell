use nu_engine::command_prelude::*;
use nu_protocol::Signals;

#[derive(Clone)]
pub struct Take;

impl Command for Take {
    fn name(&self) -> &str {
        "take"
    }

    fn signature(&self) -> Signature {
        Signature::build("take")
            .input_output_types(vec![
                (Type::table(), Type::table()),
                (
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::Any)),
                ),
                (Type::Binary, Type::Binary),
                (Type::Range, Type::List(Box::new(Type::Number))),
            ])
            .required(
                "n",
                SyntaxShape::OneOf(vec![SyntaxShape::Int, SyntaxShape::Filesize]),
                "Starting from the front, the number of elements to return.",
            )
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Take only the first n elements of a list, or the first n bytes of a binary value. For binary input, n can also be specified as a filesize."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["first", "slice", "head"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let n_val: Value = call.req(engine_state, stack, 0)?;
        let is_filesize = matches!(n_val, Value::Filesize { .. });
        let rows_desired: usize = match n_val {
            Value::Int { val, .. } => {
                usize::try_from(val).map_err(|_| ShellError::NeedsPositiveValue {
                    span: n_val.span(),
                })?
            }
            Value::Filesize { val, .. } => {
                usize::try_from(val).map_err(|_| ShellError::NeedsPositiveValue {
                    span: n_val.span(),
                })?
            }
            ref val => Err(ShellError::RuntimeTypeMismatch {
                expected: Type::custom("int or filesize"),
                actual: val.get_type(),
                span: val.span(),
            })?,
        };
        let input = input.into_stream_or_original(engine_state);

        if is_filesize {
            let is_binary = matches!(
                &input,
                PipelineData::Value(Value::Binary { .. }, _) | PipelineData::ByteStream(..)
            );
            if !is_binary {
                return Err(ShellError::IncompatibleParametersSingle {
                    msg: "Filesize is only supported for binary/byte stream input".into(),
                    span: n_val.span(),
                });
            }
        }

        match input {
            PipelineData::Value(val, metadata) => {
                let span = val.span();
                match val {
                    Value::List { vals, .. } => Ok(vals
                        .into_iter()
                        .take(rows_desired)
                        .into_pipeline_data_with_metadata(
                            head,
                            engine_state.signals().clone(),
                            metadata,
                        )),
                    Value::Binary { val, .. } => {
                        let slice: Vec<u8> = val.into_iter().take(rows_desired).collect();
                        Ok(PipelineData::value(
                            Value::binary(slice, span),
                            // first 5 bytes of an image/png stream are not image/png themselves
                            metadata.map(|m| m.with_content_type(None)),
                        ))
                    }
                    Value::Range { val, .. } => Ok(val
                        .into_range_iter(span, Signals::empty())
                        .take(rows_desired)
                        .into_pipeline_data_with_metadata(
                            head,
                            engine_state.signals().clone(),
                            metadata,
                        )),
                    // Propagate errors by explicitly matching them before the final case.
                    Value::Error { error, .. } => Err(*error),
                    other => Err(ShellError::OnlySupportsThisInputType {
                        exp_input_type: "list, binary or range".into(),
                        wrong_type: other.get_type().to_string(),
                        dst_span: head,
                        src_span: other.span(),
                    }),
                }
            }
            PipelineData::ListStream(stream, metadata) => Ok(PipelineData::list_stream(
                stream.modify(|iter| iter.take(rows_desired)),
                metadata,
            )),
            PipelineData::ByteStream(stream, metadata) => {
                if stream.type_().is_binary_coercible() {
                    let span = stream.span();
                    Ok(PipelineData::byte_stream(
                        stream.take(span, rows_desired as u64)?,
                        // first 5 bytes of an image/png stream are not image/png themselves
                        metadata.map(|m| m.with_content_type(None)),
                    ))
                } else {
                    Err(ShellError::OnlySupportsThisInputType {
                        exp_input_type: "list, binary or range".into(),
                        wrong_type: stream.type_().describe().into(),
                        dst_span: head,
                        src_span: stream.span(),
                    })
                }
            }
            PipelineData::Empty => Err(ShellError::OnlySupportsThisInputType {
                exp_input_type: "list, binary or range".into(),
                wrong_type: "null".into(),
                dst_span: head,
                src_span: head,
            }),
        }
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Return the first item of a list/table.",
                example: "[1 2 3] | take 1",
                result: Some(Value::test_list(vec![Value::test_int(1)])),
            },
            Example {
                description: "Return the first 2 items of a list/table.",
                example: "[1 2 3] | take 2",
                result: Some(Value::test_list(vec![
                    Value::test_int(1),
                    Value::test_int(2),
                ])),
            },
            Example {
                description: "Return the first two rows of a table.",
                example: "[[editions]; [2015] [2018] [2021]] | take 2",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "editions" => Value::test_int(2015),
                    }),
                    Value::test_record(record! {
                        "editions" => Value::test_int(2018),
                    }),
                ])),
            },
            Example {
                description: "Return the first 2 bytes of a binary value.",
                example: "0x[01 23 45] | take 2",
                result: Some(Value::test_binary(vec![0x01, 0x23])),
            },
            Example {
                description: "Return the first 3 elements of a range.",
                example: "1..10 | take 3",
                result: Some(Value::test_list(vec![
                    Value::test_int(1),
                    Value::test_int(2),
                    Value::test_int(3),
                ])),
            },
            Example {
                description:
                    "Return the first 3 bytes of a binary value, using a filesize argument.",
                example: "0x[01 23 45] | take 3b",
                result: Some(Value::test_binary(vec![0x01, 0x23, 0x45])),
            },
        ]
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(Take)
    }
}
