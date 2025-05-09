use nu_engine::{command_prelude::*, ClosureEvalOnce};
use nu_protocol::{engine::Closure, ListStream, Signals};

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
        let default_value: Value = call.req(engine_state, stack, 0)?;
        let columns: Vec<String> = call.rest(engine_state, stack, 1)?;
        let empty = call.has_flag(engine_state, stack, "empty")?;
        default(
            engine_state,
            stack,
            call,
            input,
            default_value,
            empty,
            columns.as_slice(),
        )
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

fn default(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
    default_value: Value,
    default_when_empty: bool,
    columns: &[String],
) -> Result<PipelineData, ShellError> {
    let input_span = input.span().unwrap_or_else(Span::unknown);
    let mut default_value = DefaultValue::new(engine_state, stack, default_value);
    let metadata = input.metadata();

    // If user supplies columns, check if input is a record or list of records
    // and set the default value for the specified record columns
    if !columns.is_empty() {
        if matches!(input, PipelineData::Value(Value::Record { .. }, _)) {
            let Value::Record {
                val: ref mut record,
                internal_span,
            } = input.into_value(input_span)?
            else {
                unreachable!()
            };
            let record = record.to_mut().into_spanned(internal_span);
            fill_record(record, &mut default_value, columns, default_when_empty)
                .map(|x| x.into_pipeline_data().set_metadata(metadata))
        } else if matches!(
            input,
            PipelineData::ListStream(..) | PipelineData::Value(Value::List { .. }, _)
        ) {
            let mut output_list: Vec<Value> = vec![];
            for mut item in input {
                if let Value::Record {
                    val: ref mut record,
                    internal_span,
                } = item
                {
                    let record = record.to_mut().into_spanned(internal_span);
                    item = fill_record(record, &mut default_value, columns, default_when_empty)?;
                    output_list.push(item);
                } else {
                    // To maintain the original functionality of `default`, we skip over
                    // non-record values in the input stream instead of returning an error
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
        default_value.pipeline_data()
    } else if default_when_empty && matches!(input, PipelineData::ListStream(..)) {
        let PipelineData::ListStream(ls, metadata) = input else {
            unreachable!()
        };
        let span = ls.span();
        let mut stream = ls.into_inner().peekable();
        if stream.peek().is_none() {
            return default_value.pipeline_data();
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

/// A wrapper around the default value to handle closures and caching values
enum DefaultValue<'a> {
    Uncalculated(&'a EngineState, &'a Stack, Spanned<Closure>),
    Calculated(Value),
}

impl<'a> DefaultValue<'a> {
    fn new(engine_state: &'a EngineState, stack: &'a Stack, value: Value) -> Self {
        let span = value.span();
        match value {
            Value::Closure { val, .. } => {
                DefaultValue::Uncalculated(engine_state, stack, (*val).into_spanned(span))
            }
            _ => DefaultValue::Calculated(value),
        }
    }

    fn value(&mut self) -> Result<Value, ShellError> {
        match self {
            DefaultValue::Uncalculated(engine_state, stack, closure) => {
                let closure_eval = ClosureEvalOnce::new(engine_state, stack, closure.item.clone());
                let value = closure_eval
                    .run_with_input(PipelineData::Empty)?
                    .into_value(closure.span)?;
                *self = DefaultValue::Calculated(value.clone());
                Ok(value)
            }
            DefaultValue::Calculated(value) => Ok(value.clone()),
        }
    }

    fn pipeline_data(&mut self) -> Result<PipelineData, ShellError> {
        self.value().map(|x| x.into_pipeline_data())
    }
}

/// Given a record, fill missing columns with a default value
fn fill_record(
    record: Spanned<&mut Record>,
    default_value: &mut DefaultValue,
    columns: &[String],
    empty: bool,
) -> Result<Value, ShellError> {
    for col in columns {
        if let Some(val) = record.item.get_mut(col) {
            if matches!(val, Value::Nothing { .. }) || (empty && val.is_empty()) {
                *val = default_value.value()?;
            }
        } else {
            record.item.push(col.clone(), default_value.value()?);
        }
    }
    Ok(Value::record(record.item.clone(), record.span))
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
