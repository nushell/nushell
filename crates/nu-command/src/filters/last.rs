use std::collections::VecDeque;

use nu_engine::CallExt;

use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData, ShellError,
    Signature, Span, SyntaxShape, Type, Value,
};

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
        let rows: Option<i64> = call.opt(engine_state, stack, 0)?;

        // FIXME: Please read the FIXME message in `first.rs`'s `first_helper` implementation.
        // It has the same issue.
        let return_single_element = rows.is_none();
        let rows_desired: usize = match rows {
            Some(i) if i < 0 => return Err(ShellError::NeedsPositiveValue { span: head }),
            Some(x) => x as usize,
            None => 1,
        };

        let ctrlc = engine_state.ctrlc.clone();
        let metadata = input.metadata();

        // early exit for `last 0`
        if rows_desired == 0 {
            return Ok(Vec::<Value>::new().into_pipeline_data_with_metadata(metadata, ctrlc));
        }

        match input {
            PipelineData::ListStream(_, _) | PipelineData::Value(Value::Range { .. }, _) => {
                let iterator = input.into_iter_strict(head)?;

                // only keep last `rows_desired` rows in memory
                let mut buf = VecDeque::<_>::new();

                for row in iterator {
                    if buf.len() == rows_desired {
                        buf.pop_front();
                    }

                    buf.push_back(row);
                }

                if return_single_element {
                    if let Some(last) = buf.pop_back() {
                        Ok(last.into_pipeline_data_with_metadata(metadata))
                    } else {
                        Ok(PipelineData::empty().set_metadata(metadata))
                    }
                } else {
                    Ok(buf.into_pipeline_data_with_metadata(metadata, ctrlc))
                }
            }
            PipelineData::Value(val, _) => {
                let val_span = val.span();

                match val {
                    Value::List { vals, .. } => {
                        if return_single_element {
                            if let Some(v) = vals.last() {
                                Ok(v.clone().into_pipeline_data())
                            } else {
                                Err(ShellError::AccessEmptyContent { span: head })
                            }
                        } else {
                            Ok(vals
                                .into_iter()
                                .rev()
                                .take(rows_desired)
                                .rev()
                                .into_pipeline_data_with_metadata(metadata, ctrlc))
                        }
                    }
                    Value::Binary { val, .. } => {
                        if return_single_element {
                            if let Some(b) = val.last() {
                                Ok(PipelineData::Value(
                                    Value::int(*b as i64, val_span),
                                    metadata,
                                ))
                            } else {
                                Err(ShellError::AccessEmptyContent { span: head })
                            }
                        } else {
                            let slice: Vec<u8> =
                                val.into_iter().rev().take(rows_desired).rev().collect();
                            Ok(PipelineData::Value(
                                Value::binary(slice, val_span),
                                metadata,
                            ))
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
