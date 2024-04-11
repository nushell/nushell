use nu_engine::command_prelude::*;

use std::collections::VecDeque;

#[derive(Clone)]
pub struct Last;

impl Command for Last {
    fn name(&self) -> &str {
        "last"
    }

    fn signature(&self) -> Signature {
        Signature::build("last")
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
                "Starting from the back, the number of rows to return.",
            )
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Return only the last several rows of the input. Counterpart of `first`. Opposite of `drop`."
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "[1,2,3] | last 2",
                description: "Return the last 2 items of a list/table",
                result: Some(Value::list(
                    vec![Value::test_int(2), Value::test_int(3)],
                    Span::test_data(),
                )),
            },
            Example {
                example: "[1,2,3] | last",
                description: "Return the last item of a list/table",
                result: Some(Value::test_int(3)),
            },
            Example {
                example: "0x[01 23 45] | last 2",
                description: "Return the last 2 bytes of a binary value",
                result: Some(Value::binary(vec![0x23, 0x45], Span::test_data())),
            },
            Example {
                example: "1..3 | last",
                description: "Return the last item of a range",
                result: Some(Value::test_int(3)),
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
        let head = call.head;
        let rows: Option<Spanned<i64>> = call.opt(engine_state, stack, 0)?;

        // FIXME: Please read the FIXME message in `first.rs`'s `first_helper` implementation.
        // It has the same issue.
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

        // early exit for `last 0`
        if rows == 0 {
            return Ok(Value::list(Vec::new(), head).into_pipeline_data_with_metadata(metadata));
        }

        match input {
            PipelineData::ListStream(_, _) | PipelineData::Value(Value::Range { .. }, _) => {
                let iterator = input.into_iter_strict(head)?;

                // only keep the last `rows` in memory
                let mut buf = VecDeque::new();

                for row in iterator {
                    if nu_utils::ctrl_c::was_pressed(&engine_state.ctrlc) {
                        return Err(ShellError::InterruptedByUser { span: Some(head) });
                    }

                    if buf.len() == rows {
                        buf.pop_front();
                    }

                    buf.push_back(row);
                }

                if return_single_element {
                    if let Some(last) = buf.pop_back() {
                        Ok(last.into_pipeline_data())
                    } else {
                        Err(ShellError::AccessEmptyContent { span: head })
                    }
                } else {
                    Ok(Value::list(buf.into(), head).into_pipeline_data_with_metadata(metadata))
                }
            }
            PipelineData::Value(val, _) => {
                let span = val.span();
                match val {
                    Value::List { mut vals, .. } => {
                        if return_single_element {
                            if let Some(v) = vals.pop() {
                                Ok(v.into_pipeline_data())
                            } else {
                                Err(ShellError::AccessEmptyContent { span: head })
                            }
                        } else {
                            let i = vals.len().saturating_sub(rows);
                            vals.drain(..i);
                            Ok(Value::list(vals, span).into_pipeline_data_with_metadata(metadata))
                        }
                    }
                    Value::Binary { mut val, .. } => {
                        if return_single_element {
                            if let Some(val) = val.pop() {
                                Ok(Value::int(val.into(), span).into_pipeline_data())
                            } else {
                                Err(ShellError::AccessEmptyContent { span: head })
                            }
                        } else {
                            let i = val.len().saturating_sub(rows);
                            val.drain(..i);
                            Ok(Value::binary(val, span).into_pipeline_data())
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
            PipelineData::ExternalStream { span, .. } => {
                Err(ShellError::OnlySupportsThisInputType {
                    exp_input_type: "list, binary or range".into(),
                    wrong_type: "raw data".into(),
                    dst_span: head,
                    src_span: span,
                })
            }
            PipelineData::Empty => Err(ShellError::OnlySupportsThisInputType {
                exp_input_type: "list, binary or range".into(),
                wrong_type: "null".into(),
                dst_span: call.head,
                src_span: call.head,
            }),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Last {})
    }
}
