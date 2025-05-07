use nu_engine::{command_prelude::*, ClosureEval, ClosureEvalOnce};
use nu_protocol::{ListStream, Signals};

#[derive(Clone)]
pub struct Default;

impl Command for Default {
    fn name(&self) -> &str {
        "default"
    }

    fn signature(&self) -> Signature {
        Signature::build("default")
            // TODO: Give more specific type signature?
            // TODO: Declare usage of cell paths in signature? (It seems to behave as if it uses cell paths)
            .input_output_types(vec![(Type::Any, Type::Any)])
            .required(
                "default value",
                SyntaxShape::Any,
                "The value to use as a default.",
            )
            .rest(
                "column name",
                SyntaxShape::String,
                "The name of the column.",
            )
            .switch(
                "empty",
                "also replace empty items like \"\", {}, and []",
                Some('e'),
            )
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Sets a default value if a row's column is missing or null."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let empty = call.has_flag(engine_state, stack, "empty")?;
        default(engine_state, stack, call, input, empty)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Give a default 'target' column to all file entries",
                example: "ls -la | default 'nothing' target ",
                result: None,
            },
            Example {
                description:
                    "Get the env value of `MY_ENV` with a default value 'abc' if not present",
                example: "$env | get --ignore-errors MY_ENV | default 'abc'",
                result: Some(Value::test_string("abc")),
            },
            Example {
                description: "Replace the `null` value in a list",
                example: "[1, 2, null, 4] | each { default 3 }",
                result: Some(Value::list(
                    vec![
                        Value::test_int(1),
                        Value::test_int(2),
                        Value::test_int(3),
                        Value::test_int(4),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: r#"Replace the missing value in the "a" column of a list"#,
                example: "[{a:1 b:2} {b:1}] | default 'N/A' a",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "a" => Value::test_int(1),
                        "b" => Value::test_int(2),
                    }),
                    Value::test_record(record! {
                        "a" => Value::test_string("N/A"),
                        "b" => Value::test_int(1),
                    }),
                ])),
            },
            Example {
                description: r#"Replace the empty string in the "a" column of a list"#,
                example: "[{a:1 b:2} {a:'' b:1}] | default -e 'N/A' a",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "a" => Value::test_int(1),
                        "b" => Value::test_int(2),
                    }),
                    Value::test_record(record! {
                        "a" => Value::test_string("N/A"),
                        "b" => Value::test_int(1),
                    }),
                ])),
            },
            Example {
                description: r#"Generate a default value from a closure"#,
                example: "null | default { 1 + 2 }",
                result: Some(Value::test_int(3)),
            },
            Example {
                description: r#"Fill missing column values based on other columns"#,
                example: r#"[{a:1 b:2} {b:1}] | upsert a {|rc| default { $rc.b + 1 } }"#,
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "a" => Value::test_int(1),
                        "b" => Value::test_int(2),
                    }),
                    Value::test_record(record! {
                        "a" => Value::test_int(2),
                        "b" => Value::test_int(1),
                    }),
                ])),
            },
        ]
    }
}

fn eval_default(
    engine_state: &EngineState,
    stack: &mut Stack,
    default_value: Value,
) -> Result<PipelineData, ShellError> {
    match &default_value {
        Value::Closure { val, .. } => {
            let closure = ClosureEvalOnce::new(engine_state, stack, *val.clone());
            closure.run_with_input(PipelineData::Empty)
        }
        _ => Ok(default_value.into_pipeline_data()),
    }
}

fn default_record_columns(
    record: &mut Record,
    default_value: Spanned<Value>,
    columns: &[String],
    empty: bool,
    engine_state: &EngineState,
    stack: &mut Stack,
    calculated_value: &mut Option<Value>,
) -> Result<PipelineData, ShellError> {
    if let Value::Closure { val: closure, .. } = &default_value.item {
        // Cache the value of the closure to avoid running it multiple times
        let mut closure = ClosureEval::new(engine_state, stack, *closure.clone());
        for col in columns {
            if let Some(val) = record.get_mut(col) {
                if matches!(val, Value::Nothing { .. }) || (empty && val.is_empty()) {
                    if let Some(ref new_value) = calculated_value {
                        *val = new_value.clone();
                    } else {
                        let new_value = closure
                            .run_with_input(PipelineData::Empty)?
                            .into_value(default_value.span)?;
                        *calculated_value = Some(new_value.clone());
                        *val = new_value;
                    }
                }
            } else if let Some(ref new_value) = calculated_value {
                record.push(col.clone(), new_value.clone());
            } else {
                let new_value = closure
                    .run_with_input(PipelineData::Empty)?
                    .into_value(default_value.span)?;
                *calculated_value = Some(new_value.clone());
                record.push(col.clone(), new_value);
            }
        }
    } else {
        for col in columns {
            if let Some(val) = record.get_mut(col) {
                if matches!(val, Value::Nothing { .. }) || (empty && val.is_empty()) {
                    *val = default_value.item.clone();
                }
            } else {
                record.push(col.clone(), default_value.item.clone());
            }
        }
    }
    Ok(Value::record(record.clone(), Span::unknown()).into_pipeline_data())
}

fn default(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
    default_when_empty: bool,
) -> Result<PipelineData, ShellError> {
    let input_span = input.span().unwrap_or_else(Span::unknown);
    let metadata = input.metadata();
    let default_value: Spanned<Value> = call.req(engine_state, stack, 0)?;
    let columns: Vec<String> = call.rest(engine_state, stack, 1)?;

    // If user supplies columns, check if input is a record or list of records
    // and set the default value for the specified record columns
    if !columns.is_empty() {
        // Single record arm
        if matches!(input, PipelineData::Value(Value::Record { .. }, _)) {
            let Value::Record {
                val: ref mut record,
                ..
            } = input.into_value(input_span)?
            else {
                unreachable!()
            };
            let record = record.to_mut();
            default_record_columns(
                record,
                default_value,
                columns.as_slice(),
                default_when_empty,
                engine_state,
                stack,
                &mut None,
            )
            .map(|x| x.set_metadata(metadata))
        // ListStream arm
        } else if matches!(input, PipelineData::ListStream(..))
            || matches!(input, PipelineData::Value(Value::List { .. }, _))
        {
            let mut calculated_value: Option<Value> = None;
            let mut output_list: Vec<Value> = vec![];
            for mut item in input {
                if let Value::Record {
                    val: ref mut record,
                    internal_span,
                } = item
                {
                    let item = default_record_columns(
                        record.to_mut(),
                        default_value.clone(),
                        columns.as_slice(),
                        default_when_empty,
                        engine_state,
                        stack,
                        &mut calculated_value,
                    )?;
                    output_list.push(item.into_value(internal_span)?);
                } else {
                    output_list.push(item);
                }
            }
            let ls = ListStream::new(
                output_list.into_iter(),
                call.head,
                engine_state.signals().clone(),
            );
            Ok(PipelineData::ListStream(ls, metadata))
        // If columns are given, but input does not use columns, return an error
        } else {
            Err(ShellError::PipelineMismatch {
                exp_input_type: "record, table".to_string(),
                dst_span: input_span,
                src_span: input_span,
            })
        }
    // Otherwise, if no column name is given, check if value is null
    // or an empty string, list, or record when --empty is passed
    } else if input.is_nothing()
        || (default_when_empty
            && matches!(input, PipelineData::Value(ref value, _) if value.is_empty()))
    {
        eval_default(engine_state, stack, default_value.item)
    } else if default_when_empty && matches!(input, PipelineData::ListStream(..)) {
        let PipelineData::ListStream(ls, metadata) = input else {
            unreachable!()
        };
        let span = ls.span();
        let mut stream = ls.into_inner().peekable();
        if stream.peek().is_none() {
            return eval_default(engine_state, stack, default_value.item);
        }

        // stream's internal state already preserves the original signals config, so if this
        // Signals::empty list stream gets interrupted it will be caught by the underlying iterator
        let ls = ListStream::new(stream, span, Signals::empty());
        Ok(PipelineData::ListStream(ls, metadata))
    // Otherwise, return the input as is
    } else {
        Ok(input)
    }
}

#[cfg(test)]
mod test {
    use crate::Upsert;

    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples_with_commands;

        test_examples_with_commands(Default {}, &[&Upsert]);
    }
}
