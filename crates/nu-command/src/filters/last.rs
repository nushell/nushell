#[cfg(feature = "sqlite")]
use crate::database::QueryPlan;
use crate::math::utils::ensure_bounded;
use nu_engine::command_prelude::*;
use nu_protocol::{Signals, shell_error::bridge::ShellErrorBridge};
use std::{collections::VecDeque, io::Read};

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
                (Type::Binary, Type::one_of([Type::Int, Type::Binary])),
                (Type::Range, Type::Any),
            ])
            .optional(
                "rows",
                SyntaxShape::OneOf(vec![SyntaxShape::Int, SyntaxShape::Filesize]),
                "Starting from the back, the number of rows to return.",
            )
            .allow_variants_without_examples(true)
            .switch("strict", "Throw an error if input is empty.", Some('s'))
            .category(Category::Filters)
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["tail", "end"]
    }

    fn description(&self) -> &str {
        "Return only the last several rows of the input. Counterpart of `first`. Opposite of `drop`. For binary input, rows can also be specified as a filesize."
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: "[1,2,3] | last 2",
                description: "Return the last 2 items of a list/table.",
                result: Some(Value::list(
                    vec![Value::test_int(2), Value::test_int(3)],
                    Span::test_data(),
                )),
            },
            Example {
                example: "[1,2,3] | last",
                description: "Return the last item of a list/table.",
                result: Some(Value::test_int(3)),
            },
            Example {
                example: "0x[01 23 45] | last",
                description: "Return the last byte of a binary value as an int.",
                result: Some(Value::test_int(0x45)),
            },
            Example {
                example: "0x[01 23 45] | last 2",
                description: "Return the last 2 bytes of a binary value.",
                result: Some(Value::binary(vec![0x23, 0x45], Span::test_data())),
            },
            Example {
                example: "1..3 | last",
                description: "Return the last item of a range.",
                result: Some(Value::test_int(3)),
            },
            Example {
                example: "0x[01 23 45] | last 2b",
                description: "Return the last 2 bytes of a binary value, using a filesize argument.",
                result: Some(Value::test_binary(vec![0x23, 0x45])),
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
        let rows_val: Option<Value> = call.opt(engine_state, stack, 0)?;
        let strict_mode = call.has_flag(engine_state, stack, "strict")?;
        last_helper(
            rows_val,
            strict_mode,
            engine_state.signals(),
            call.head,
            input,
        )
    }

    fn is_const(&self) -> bool {
        true
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let rows_val: Option<Value> = call.opt_const(working_set, stack, 0)?;
        let strict_mode = call.has_flag_const(working_set, stack, "strict")?;
        last_helper(
            rows_val,
            strict_mode,
            working_set.permanent().signals(),
            call.head,
            input,
        )
    }
}

/// Body of `last`, shared by `run` and `run_const`: the last `rows_val` items of `input`, or the last item when `rows_val` is `None`.
fn last_helper(
    rows_val: Option<Value>,
    strict_mode: bool,
    signals: &Signals,
    head: Span,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let is_filesize = rows_val
        .as_ref()
        .is_some_and(|v| matches!(v, Value::Filesize { .. }));

    let rows: Option<usize> = match rows_val {
        Some(v) => {
            let span = v.span();
            match v {
                Value::Int { val, .. } => Some(
                    usize::try_from(val).map_err(|_| ShellError::NeedsPositiveValue { span })?,
                ),
                Value::Filesize { val, .. } => Some(
                    usize::try_from(val).map_err(|_| ShellError::NeedsPositiveValue { span })?,
                ),
                ref val => {
                    return Err(ShellError::RuntimeTypeMismatch {
                        expected: Type::custom("int or filesize"),
                        actual: val.get_type(),
                        span: val.span(),
                    });
                }
            }
        }
        None => None,
    };

    // FIXME: Please read the FIXME message in `first.rs`'s `first_helper` implementation.
    // It has the same issue.
    let return_single_element = rows.is_none();
    let rows = rows.unwrap_or(1);

    let mut input = input;
    let metadata = input.take_metadata();

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

    // Count is 0: return empty data immediately.
    //
    // The main `match` below is not safe for this case-`last` reads binary streams in chunks
    // and sqlite paths may still execute. For "take nothing" we only produce an empty value:
    // empty binary (and clear pipeline `content_type` for binary) or an empty list, with other
    // metadata unchanged.
    //
    // With `--strict`, `-s 0` still has to look at the first item (or query with `LIMIT 1`)
    // to tell an empty input apart, so it goes on to the main `match`.
    if rows == 0 && !strict_mode {
        return match input {
            PipelineData::Value(val, _) if matches!(&val, Value::Binary { .. }) => {
                Ok(Value::binary(Vec::new(), val.span())
                    .into_pipeline_data_with_metadata(metadata.map(|m| m.with_content_type(None))))
            }
            PipelineData::ByteStream(stream, _) => {
                if stream.type_().is_binary_coercible() {
                    let span = stream.span();
                    Ok(
                        Value::binary(Vec::new(), span).into_pipeline_data_with_metadata(
                            metadata.map(|m| m.with_content_type(None)),
                        ),
                    )
                } else {
                    Ok(Value::list(Vec::new(), head).into_pipeline_data_with_metadata(metadata))
                }
            }
            _ => Ok(Value::list(Vec::new(), head).into_pipeline_data_with_metadata(metadata)),
        };
    }

    if let PipelineData::Value(ref v @ Value::Range { ref val, .. }, _) = input
        && rows > 0
    {
        ensure_bounded(val, v.span(), head)?;
    }

    // Take this before `match` consumes the input. An external that fails without
    // output then reports its exit code instead of empty content.
    #[cfg(feature = "os")]
    let exit_status = input.clone_exit_status_future();

    match input {
        PipelineData::ListStream(_, _) | PipelineData::Value(Value::Range { .. }, _) => {
            let iterator = input.into_iter_strict(head)?;

            // only keep the last `rows` in memory
            // With `--strict` and `rows == 0`, we need at least 1 slot to see if the input is non-empty.
            let cap = if rows == 0 && strict_mode { 1 } else { rows };
            let mut buf = VecDeque::new();

            for row in iterator {
                signals.check(&head)?;
                // Every row gets read, so an error counts even in a row that won't be returned.
                let row = row.unwrap_error()?;
                if buf.len() == cap {
                    buf.pop_front();
                }
                buf.push_back(row);
                if rows == 0 {
                    break;
                }
            }

            if return_single_element {
                if let Some(last) = buf.pop_back() {
                    Ok(last.into_pipeline_data_with_metadata(metadata))
                } else if strict_mode {
                    Err(ShellError::AccessEmptyContent { span: head })
                } else {
                    // There are no values, so return nothing instead of an error so
                    // that users can pipe this through 'default' if they want to.
                    Ok(Value::nothing(head).into_pipeline_data_with_metadata(metadata))
                }
            } else if strict_mode && buf.is_empty() {
                Err(ShellError::AccessEmptyContent { span: head })
            } else if rows == 0 {
                Ok(Value::list(Vec::new(), head).into_pipeline_data_with_metadata(metadata))
            } else {
                Ok(Value::list(buf.into(), head).into_pipeline_data_with_metadata(metadata))
            }
        }
        PipelineData::Value(val, _) => {
            let span = val.span();
            match val {
                Value::List { vals, .. } => {
                    if return_single_element {
                        if let Some(v) = vals.last() {
                            Ok(v.clone().into_pipeline_data_with_metadata(metadata))
                        } else if strict_mode {
                            Err(ShellError::AccessEmptyContent { span: head })
                        } else {
                            // There are no values, so return nothing instead of an error so
                            // that users can pipe this through 'default' if they want to.
                            Ok(Value::nothing(head).into_pipeline_data_with_metadata(metadata))
                        }
                    } else if strict_mode && vals.is_empty() {
                        Err(ShellError::AccessEmptyContent { span: head })
                    } else if rows == 0 {
                        Ok(
                            Value::list(Vec::new(), head)
                                .into_pipeline_data_with_metadata(metadata),
                        )
                    } else {
                        let i = vals.len().saturating_sub(rows);
                        let value = if i == 0 {
                            Value::list_shared(vals, span)
                        } else {
                            Value::list(vals.iter().skip(i).cloned().collect(), span)
                        };
                        Ok(value.into_pipeline_data_with_metadata(metadata))
                    }
                }
                Value::Binary { val, .. } => {
                    let binary_meta = metadata.map(|m| m.with_content_type(None));
                    if return_single_element {
                        if let Some(&val) = val.last() {
                            Ok(Value::int(val.into(), span)
                                .into_pipeline_data_with_metadata(binary_meta))
                        } else if strict_mode {
                            Err(ShellError::AccessEmptyContent { span: head })
                        } else {
                            // There are no values, so return nothing instead of an error so
                            // that users can pipe this through 'default' if they want to.
                            Ok(Value::nothing(head).into_pipeline_data_with_metadata(binary_meta))
                        }
                    } else if strict_mode && val.is_empty() {
                        Err(ShellError::AccessEmptyContent { span: head })
                    } else if rows == 0 {
                        Ok(Value::binary(Vec::new(), span)
                            .into_pipeline_data_with_metadata(binary_meta))
                    } else {
                        let mut val = val.into_owned();
                        let i = val.len().saturating_sub(rows);
                        val.drain(..i);
                        Ok(Value::binary(val, span).into_pipeline_data_with_metadata(binary_meta))
                    }
                }
                // Propagate errors by explicitly matching them before the final case.
                Value::Error { error, .. } => Err(*error),
                #[cfg(feature = "sqlite")]
                // Pushdown optimization: handle 'last' via QueryPlan for lazy SQL execution
                Value::Custom {
                    val: custom_val,
                    internal_span,
                    ..
                } => {
                    if let Some(plan) = QueryPlan::try_from_any(custom_val.as_any()) {
                        let plan = plan
                            .with_order_by("rowid DESC".to_string())
                            .with_limit(rows.max(1) as i64);
                        let value = plan.execute(head)?.into_value(head)?;
                        let Value::List { mut vals, .. } = value else {
                            return Ok(value.into_pipeline_data_with_metadata(metadata));
                        };
                        if strict_mode && vals.is_empty() {
                            return Err(ShellError::AccessEmptyContent { span: head });
                        }
                        let value = if return_single_element {
                            vals.into_owned()
                                .into_iter()
                                .next()
                                .unwrap_or_else(|| Value::nothing(head))
                        } else if rows == 0 {
                            Value::list(Vec::new(), head)
                        } else {
                            vals.to_mut().reverse();
                            Value::list(vals.into_owned(), head)
                        };
                        Ok(value.into_pipeline_data_with_metadata(metadata))
                    } else {
                        Err(ShellError::OnlySupportsThisInputType {
                            exp_input_type: "list, binary or range".into(),
                            wrong_type: custom_val.type_name(),
                            dst_span: head,
                            src_span: internal_span,
                        })
                    }
                }
                other => Err(ShellError::OnlySupportsThisInputType {
                    exp_input_type: "list, binary or range".into(),
                    wrong_type: other.get_type().to_string(),
                    dst_span: head,
                    src_span: other.span(),
                }),
            }
        }
        PipelineData::ByteStream(stream, ..) => {
            if stream.type_().is_binary_coercible() {
                let span = stream.span();
                let byte_meta = metadata.map(|m| m.with_content_type(None));
                if let Some(mut reader) = stream.reader() {
                    // `-s 0` (only `--strict` gets here with 0) needs one byte to tell empty
                    // input apart. On 0 bytes the loop below reaches EOF and reports it.
                    if rows == 0 {
                        let mut byte = [0u8];
                        if reader
                            .read(&mut byte)
                            .map_err(|err| ShellErrorBridge::unpack(err, span))?
                            > 0
                        {
                            return Ok(Value::binary(Vec::new(), span)
                                .into_pipeline_data_with_metadata(byte_meta));
                        }
                    }
                    // Have to be a bit tricky here, but just consume into a VecDeque that we
                    // shrink to fit each time
                    const TAKE: u64 = 8192;
                    // `rows` is user supplied and can be far larger than the input, so don't reserve it.
                    let mut buf = VecDeque::new();
                    loop {
                        let taken = std::io::copy(&mut (&mut reader).take(TAKE), &mut buf)
                            .map_err(|err| ShellErrorBridge::unpack(err, span))?;
                        if buf.len() > rows {
                            buf.drain(..(buf.len() - rows));
                        }
                        if taken < TAKE {
                            // This must be EOF.
                            if strict_mode && buf.is_empty() {
                                #[cfg(feature = "os")]
                                nu_protocol::process::check_exit_status_future(vec![
                                    exit_status.map(|status| status.with_span(span)),
                                ])?;
                                return Err(ShellError::AccessEmptyContent { span: head });
                            }
                            if return_single_element {
                                if !buf.is_empty() {
                                    return Ok(Value::int(buf[0] as i64, head)
                                        .into_pipeline_data_with_metadata(byte_meta));
                                } else {
                                    // There are no values, so return nothing instead of an error so
                                    // that users can pipe this through 'default' if they want to.
                                    return Ok(Value::nothing(head)
                                        .into_pipeline_data_with_metadata(byte_meta));
                                }
                            } else {
                                return Ok(Value::binary(buf, head)
                                    .into_pipeline_data_with_metadata(byte_meta));
                            }
                        }
                    }
                } else if strict_mode {
                    #[cfg(feature = "os")]
                    nu_protocol::process::check_exit_status_future(vec![
                        exit_status.map(|status| status.with_span(span)),
                    ])?;
                    Err(ShellError::AccessEmptyContent { span: head })
                } else {
                    Ok(Value::nothing(head).into_pipeline_data_with_metadata(byte_meta))
                }
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(Last)
    }
}
