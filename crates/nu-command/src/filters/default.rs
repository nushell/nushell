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
            .optional(
                "column name",
                SyntaxShape::String,
                "The name of the column.",
            )
            .switch(
                "empty",
                "also replace empty items like \"\", {}, and []",
                Some('e'),
            )
            .switch(
                "lazy",
                "if default value is a closure, evaluate it",
                Some('l'),
            )
            .switch(
                "lazy-once",
                "evaluate the closure only once, even for lists (no input)",
                Some('L'),
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
        let lazy = call.has_flag(engine_state, stack, "lazy")?;
        let lazy_once = call.has_flag(engine_state, stack, "lazy-once")?;
        default(engine_state, stack, call, input, empty, lazy, lazy_once)
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
                example: "null | default --lazy { 1 + 2 }",
                result: Some(Value::test_int(3)),
            },
            Example {
                description: r#"Generate missing values in a column from a closure"#,
                example: "[{a:1 b:2} {b:1}] | default -l { $in.b + 1 } a",
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

fn default_value_or_eval_once(
    engine_state: &EngineState,
    stack: &mut Stack,
    input: PipelineData,
    default_value: Value,
    lazy: bool,
) -> Result<PipelineData, ShellError> {
    match (&default_value, lazy) {
        (Value::Closure { val, .. }, true) => {
            let closure = ClosureEvalOnce::new(engine_state, stack, *val.clone());
            closure.run_with_input(input)
        }
        _ => Ok(default_value.into_pipeline_data()),
    }
}

fn default(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
    default_when_empty: bool,
    lazy_eval: bool,
    lazy_eval_once: bool,
) -> Result<PipelineData, ShellError> {
    let metadata = input.metadata();
    let value: Value = call.req(engine_state, stack, 0)?;
    let column: Option<Spanned<String>> = call.opt(engine_state, stack, 1)?;

    if let Some(column) = column {
        if lazy_eval && !lazy_eval_once && matches!(value, Value::Closure { .. }) {
            let Value::Closure {
                val: ref closure,
                internal_span: closure_span,
            } = value
            else {
                unreachable!()
            };
            let mut closure = ClosureEval::new(engine_state, stack, *closure.clone());
            input
                .map(
                    move |mut item| match item {
                        Value::Record {
                            val: ref mut record,
                            internal_span: record_span,
                        } => {
                            let closure_input = record.clone().into_owned();
                            let record = record.to_mut();
                            if let Some(val) = record.get_mut(&column.item) {
                                if matches!(val, Value::Nothing { .. })
                                    || (default_when_empty && val.is_empty())
                                {
                                    *val = match closure
                                        .run_with_value(Value::record(closure_input, record_span))
                                    {
                                        Ok(value) => value
                                            .into_value(closure_span)
                                            .unwrap_or_else(|err| Value::error(err, closure_span)),
                                        Err(err) => Value::error(err, closure_span),
                                    };
                                }
                            } else {
                                let new_value = match closure
                                    .run_with_value(Value::record(closure_input, record_span))
                                {
                                    Ok(value) => value
                                        .into_value(closure_span)
                                        .unwrap_or_else(|err| Value::error(err, closure_span)),
                                    Err(err) => Value::error(err, closure_span),
                                };
                                record.push(column.item.clone(), new_value);
                            }

                            item
                        }
                        _ => item,
                    },
                    engine_state.signals(),
                )
                .map(|x| x.set_metadata(metadata))
        } else {
            let value_span = value.span();
            let value = default_value_or_eval_once(
                engine_state,
                stack,
                PipelineData::Empty,
                value,
                lazy_eval_once,
            )?
            .into_value(value_span)?;

            input
                .map(
                    move |mut item| match item {
                        Value::Record {
                            val: ref mut record,
                            ..
                        } => {
                            let record = record.to_mut();
                            if let Some(val) = record.get_mut(&column.item) {
                                if matches!(val, Value::Nothing { .. })
                                    || (default_when_empty && val.is_empty())
                                {
                                    *val = value.clone();
                                }
                            } else {
                                record.push(column.item.clone(), value.clone());
                            }

                            item
                        }
                        _ => item,
                    },
                    engine_state.signals(),
                )
                .map(|x| x.set_metadata(metadata))
        }
    } else if input.is_nothing()
        || (default_when_empty
            && matches!(input, PipelineData::Value(ref value, _) if value.is_empty()))
    {
        default_value_or_eval_once(
            engine_state,
            stack,
            input,
            value,
            lazy_eval || lazy_eval_once,
        )
    } else if default_when_empty && matches!(input, PipelineData::ListStream(..)) {
        let PipelineData::ListStream(ls, metadata) = input else {
            unreachable!()
        };
        let span = ls.span();
        let mut stream = ls.into_inner().peekable();
        if stream.peek().is_none() {
            return default_value_or_eval_once(
                engine_state,
                stack,
                PipelineData::Empty,
                value,
                lazy_eval || lazy_eval_once,
            );
        }

        // stream's internal state already preserves the original signals config, so if this
        // Signals::empty list stream gets interrupted it will be caught by the underlying iterator
        let ls = ListStream::new(stream, span, Signals::empty());
        Ok(PipelineData::ListStream(ls, metadata))
    } else {
        Ok(input)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Default {})
    }
}
