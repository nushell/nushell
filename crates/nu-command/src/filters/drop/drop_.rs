use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct Drop;

impl Command for Drop {
    fn name(&self) -> &str {
        "drop"
    }

    fn signature(&self) -> Signature {
        Signature::build("drop")
            .input_output_types(vec![
                (Type::table(), Type::table()),
                (
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::Any)),
                ),
                (Type::Binary, Type::Binary),
            ])
            .optional(
                "rows",
                SyntaxShape::OneOf(vec![SyntaxShape::Int, SyntaxShape::Filesize]),
                "The number of items to remove.",
            )
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Remove items/rows from the end of the input list/table, or remove bytes from the end of binary data. Counterpart of `skip`. Opposite of `last`. For binary input, `rows` can also be specified as a filesize."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["delete", "remove"]
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: "[0,1,2,3] | drop",
                description: "Remove the last item of a list",
                result: Some(Value::test_list(vec![
                    Value::test_int(0),
                    Value::test_int(1),
                    Value::test_int(2),
                ])),
            },
            Example {
                example: "[0,1,2,3] | drop 0",
                description: "Remove zero item of a list",
                result: Some(Value::test_list(vec![
                    Value::test_int(0),
                    Value::test_int(1),
                    Value::test_int(2),
                    Value::test_int(3),
                ])),
            },
            Example {
                example: "[0,1,2,3] | drop 2",
                description: "Remove the last two items of a list",
                result: Some(Value::test_list(vec![
                    Value::test_int(0),
                    Value::test_int(1),
                ])),
            },
            Example {
                description: "Remove the last row in a table",
                example: "[[a, b]; [1, 2] [3, 4]] | drop 1",
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "a" => Value::test_int(1),
                    "b" => Value::test_int(2),
                })])),
            },
            Example {
                example: "0x[01 23 45] | drop 2b",
                description: "Remove the last 2 bytes of a binary value, using a filesize argument",
                result: Some(Value::test_binary(vec![0x01])),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        mut input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let metadata = input.take_metadata();

        let rows_val: Option<Value> = call.opt(engine_state, stack, 0)?;
        let is_filesize = rows_val
            .as_ref()
            .is_some_and(|v| matches!(v, Value::Filesize { .. }));

        let rows: usize = match rows_val {
            Some(v) => {
                let span = v.span();
                match v {
                    Value::Int { val, .. } => {
                        usize::try_from(val).map_err(|_| ShellError::NeedsPositiveValue { span })?
                    }
                    Value::Filesize { val, .. } => {
                        usize::try_from(val).map_err(|_| ShellError::NeedsPositiveValue { span })?
                    }
                    ref val => {
                        return Err(ShellError::RuntimeTypeMismatch {
                            expected: Type::custom("int or filesize"),
                            actual: val.get_type(),
                            span: val.span(),
                        });
                    }
                }
            }
            None => 1,
        };

        if is_filesize {
            let is_binary = matches!(
                &input,
                PipelineData::Value(Value::Binary { .. }, _) | PipelineData::ByteStream(..)
            );
            if !is_binary {
                return Err(ShellError::IncompatibleParametersSingle {
                    msg: "Filesize is only supported for binary/byte stream input".into(),
                    span: head,
                });
            }
        }

        match input {
            PipelineData::Value(Value::Binary { val, .. }, ..) => {
                let len = val.len();
                let take = len.saturating_sub(rows);
                Ok(Value::binary(&val[..take], head)
                    .into_pipeline_data_with_metadata(metadata))
            }
            PipelineData::ByteStream(stream, ..) => {
                let bytes = stream.into_bytes()?;
                let len = bytes.len();
                let take = len.saturating_sub(rows);
                Ok(Value::binary(&bytes[..take], head)
                    .into_pipeline_data_with_metadata(metadata))
            }
            _ => {
                let mut values = input.into_iter_strict(head)?.collect::<Vec<_>>();
                values.truncate(values.len().saturating_sub(rows));
                Ok(Value::list(values, head).into_pipeline_data_with_metadata(metadata))
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::Drop;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(Drop)
    }
}
