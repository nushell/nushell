use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct First;

impl Command for First {
    fn name(&self) -> &str {
        "first"
    }

    fn signature(&self) -> Signature {
        Signature::build("first")
            .input_output_types(vec![
                (
                    // TODO: This is too permissive; if we could express this
                    // using a type parameter it would be List<T> -> T.
                    Type::List(Box::new(Type::Any)),
                    Type::Any,
                ),
                (Type::Binary, Type::Binary),
                (Type::Range, Type::Any),
            ])
            .optional(
                "rows",
                SyntaxShape::Int,
                "Starting from the front, the number of rows to return.",
            )
            .allow_variants_without_examples(true)
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Return only the first several rows of the input. Counterpart of `last`. Opposite of `skip`."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        first_helper(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Return the first item of a list/table",
                example: "[1 2 3] | first",
                result: Some(Value::test_int(1)),
            },
            Example {
                description: "Return the first 2 items of a list/table",
                example: "[1 2 3] | first 2",
                result: Some(Value::list(
                    vec![Value::test_int(1), Value::test_int(2)],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Return the first 2 bytes of a binary value",
                example: "0x[01 23 45] | first 2",
                result: Some(Value::binary(vec![0x01, 0x23], Span::test_data())),
            },
            Example {
                description: "Return the first item of a range",
                example: "1..3 | first",
                result: Some(Value::test_int(1)),
            },
        ]
    }
}

fn first_helper(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let head = call.head;
    let rows: Option<Spanned<i64>> = call.opt(engine_state, stack, 0)?;

    // FIXME: for backwards compatibility reasons, if `rows` is not specified we
    // return a single element and otherwise we return a single list. We should probably
    // remove `rows` so that `first` always returns a single element; getting a list of
    // the first N elements is covered by `take`
    let return_single_element = rows.is_none();
    let rows = if let Some(rows) = rows {
        if rows.item < 0 {
            return Err(ShellError::NeedsPositiveValue { span: rows.span });
        } else {
            rows.item as usize
        }
    } else {
        1
    };

    let metadata = input.metadata();

    // early exit for `first 0`
    if rows == 0 {
        return Ok(Value::list(Vec::new(), head).into_pipeline_data_with_metadata(metadata));
    }

    match input {
        PipelineData::Value(val, _) => {
            let span = val.span();
            match val {
                Value::List { mut vals, .. } => {
                    if return_single_element {
                        if let Some(val) = vals.first_mut() {
                            Ok(std::mem::take(val).into_pipeline_data())
                        } else {
                            Err(ShellError::AccessEmptyContent { span: head })
                        }
                    } else {
                        vals.truncate(rows);
                        Ok(Value::list(vals, span).into_pipeline_data_with_metadata(metadata))
                    }
                }
                Value::Binary { mut val, .. } => {
                    if return_single_element {
                        if let Some(&val) = val.first() {
                            Ok(Value::int(val.into(), span).into_pipeline_data())
                        } else {
                            Err(ShellError::AccessEmptyContent { span: head })
                        }
                    } else {
                        val.truncate(rows);
                        Ok(Value::binary(val, span).into_pipeline_data_with_metadata(metadata))
                    }
                }
                Value::Range { val, .. } => {
                    let ctrlc = engine_state.ctrlc.clone();
                    let mut iter = val.into_range_iter(span, ctrlc.clone());
                    if return_single_element {
                        if let Some(v) = iter.next() {
                            Ok(v.into_pipeline_data())
                        } else {
                            Err(ShellError::AccessEmptyContent { span: head })
                        }
                    } else {
                        Ok(iter
                            .take(rows)
                            .into_pipeline_data_with_metadata(metadata, ctrlc))
                    }
                }
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
        PipelineData::ListStream(mut ls, metadata) => {
            if return_single_element {
                if let Some(v) = ls.next() {
                    Ok(v.into_pipeline_data())
                } else {
                    Err(ShellError::AccessEmptyContent { span: head })
                }
            } else {
                Ok(ls
                    .take(rows)
                    .into_pipeline_data_with_metadata(metadata, engine_state.ctrlc.clone()))
            }
        }
        PipelineData::ExternalStream { span, .. } => Err(ShellError::OnlySupportsThisInputType {
            exp_input_type: "list, binary or range".into(),
            wrong_type: "raw data".into(),
            dst_span: head,
            src_span: span,
        }),
        PipelineData::Empty => Err(ShellError::OnlySupportsThisInputType {
            exp_input_type: "list, binary or range".into(),
            wrong_type: "null".into(),
            dst_span: call.head,
            src_span: call.head,
        }),
    }
}
#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(First {})
    }
}
