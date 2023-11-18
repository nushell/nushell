use nu_protocol::{
    ast::Call,
    engine::{Closure, Command, EngineState, Stack, StateWorkingSet},
    record, Category, Example, IntoPipelineData, PipelineData, PipelineMetadata, Record,
    ShellError, Signature, Type, Value,
};

#[derive(Clone)]
pub struct Describe;

impl Command for Describe {
    fn name(&self) -> &str {
        "describe"
    }

    fn usage(&self) -> &str {
        "Describe the type and structure of the value(s) piped in."
    }

    fn signature(&self) -> Signature {
        Signature::build("describe")
            .input_output_types(vec![
                (Type::Any, Type::Record(vec![])),
                (Type::Any, Type::String),
            ])
            .filter()
            .switch(
                "no-collect",
                "do not collect streams of structured data",
                Some('n'),
            )
            .switch(
                "detailed",
                "show detailed information about the value",
                Some('d'),
            )
            .switch("collect-lazyrecords", "collect lazy records", Some('l'))
            .category(Category::Core)
    }

    fn is_const(&self) -> bool {
        true
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        run(Some(engine_state), call, input)
    }

    fn run_const(
        &self,
        _working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        run(None, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Describe the type of a string",
                example: "'hello' | describe",
                result: Some(Value::test_string("string")),
            },
            Example {
                description: "Describe the type of a record in a detailed way",
                example:
                    "{shell:'true', uwu:true, features: {bugs:false, multiplatform:true, speed: 10}, fib: [1 1 2 3 5 8], on_save: {|x| print $'Saving ($x)'}, first_commit: 2019-05-10, my_duration: (4min + 20sec)} | describe -d",
                result: Some(Value::test_record(record!(
                    "type" => Value::test_type_literal(Type::Record(vec![
                        ("shell".to_string(), Type::String),
                        ("uwu".to_string(), Type::Bool),
                        ("features".to_string(), Type::Record(vec![
                            ("bugs".to_string(), Type::Bool),
                            ("multiplatform".to_string(), Type::Bool),
                            ("speed".to_string(), Type::Int),
                        ])),
                        ("fib".to_string(), Type::List(Box::new(Type::Int))),
                        ("on_save".to_string(), Type::Closure),
                        ("first_commit".to_string(), Type::Date),
                        ("my_duration".to_string(), Type::Duration),
                    ])),
                    "lazy" => Value::test_bool(false),
                    "columns" => Value::test_record(record!(
                        "shell" => Value::test_type_literal(Type::String),
                        "uwu" => Value::test_type_literal(Type::Bool),
                        "features" => Value::test_type_literal(Type::Record(vec![
                            ("bugs".to_string(), Type::Bool),
                            ("multiplatform".to_string(), Type::Bool),
                            ("speed".to_string(), Type::Int),
                        ])),
                        "fib" => Value::test_type_literal(Type::List(Box::new(Type::Int))),
                        "on_save" => Value::test_type_literal(Type::Closure),
                        "first_commit" => Value::test_type_literal(Type::Date),
                        "my_duration" => Value::test_type_literal(Type::Duration),
                    )),
                ))),
            },
            Example {
                description: "Describe the type of a stream with detailed information",
                example: "[1 2 3] | each {|i| echo $i} | describe -d",
                result: None // Give "Running external commands not supported" error
                // result: Some(Value::test_record(record!(
                //     "type" => Value::test_string("stream"),
                //     "origin" => Value::test_string("nushell"),
                //     "subtype" => Value::test_record(record!(
                //         "type" => Value::test_string("list"),
                //         "length" => Value::test_int(3),
                //         "values" => Value::test_list(vec![
                //             Value::test_string("int"),
                //             Value::test_string("int"),
                //             Value::test_string("int"),
                //         ])
                //     ))
                // ))),
            },
            Example {
                description: "Describe a stream of data, collecting it first",
                example: "[1 2 3] | each {|i| echo $i} | describe",
                result: None // Give "Running external commands not supported" error
                // result: Some(Value::test_string("list<int> (stream)")),
            },
            Example {
                description: "Describe the input but do not collect streams",
                example: "[1 2 3] | each {|i| echo $i} | describe --no-collect",
                result: None // Give "Running external commands not supported" error
                // result: Some(Value::test_string("stream")),
            },
        ]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["type", "typeof", "info", "structure"]
    }
}

fn run(
    engine_state: Option<&EngineState>,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let metadata = input.metadata().clone().map(Box::new);
    let head = call.head;
    let no_collect: bool = call.has_flag("no-collect");
    let detailed = call.has_flag("detailed");

    let description: Value = match input {
        PipelineData::ExternalStream {
            ref stdout,
            ref stderr,
            ref exit_code,
            ..
        } => {
            if detailed {
                Value::record(
                    record!(
                        "type" => Value::type_literal(Type::ListStream, head),
                        "origin" => Value::string("external", head),
                        "stdout" => match stdout {
                            Some(_) => Value::record(
                                    record!(
                                        "type" => Value::type_literal(Type::ListStream, head),
                                        "origin" => Value::string("external", head),
                                        "subtype" => Value::string("any", head),
                                    ),
                                    head,
                                ),
                            None => Value::nothing(head),
                        },
                        "stderr" => match stderr {
                            Some(_) => Value::record(
                                    record!(
                                        "type" => Value::type_literal(Type::ListStream, head),
                                        "origin" => Value::string("external", head),
                                        "subtype" => Value::string("any", head),
                                    ),
                                    head,
                                ),
                            None => Value::nothing(head),
                        },
                        "exit_code" => match exit_code {
                            Some(_) => Value::record(
                                    record!(
                                        "type" => Value::type_literal(Type::ListStream, head),
                                        "origin" => Value::string("external", head),
                                        "subtype" => Value::string("int", head),
                                    ),
                                    head,
                                ),
                            None => Value::nothing(head),
                        },
                        "metadata" => metadata_to_value(metadata, head),
                    ),
                    head,
                )
            } else {
                Value::string("raw input", head)
            }
        }
        PipelineData::ListStream(_, _) => {
            if detailed {
                let (ty, subty) = if no_collect {
                    (Type::ListStream, Value::nothing(head))
                } else {
                    let value = input.into_value(head);
                    let ty = value.get_type().clone();
                    let desc = describe_value(value, head, engine_state, call)?;
                    (ty, desc)
                };
                Value::record(
                    record!(
                        "type" => Value::type_literal(ty, head),
                        "origin" => Value::string("nushell", head),
                        "description" => subty,
                        "metadata" => metadata_to_value(metadata, head),
                    ),
                    head,
                )
            } else if no_collect {
                Value::string("stream", head)
            } else {
                let value = input.into_value(head);
                let base_description = match value {
                    Value::CustomValue { val, .. } => val.value_string(),
                    _ => value.get_type().to_string(),
                };

                Value::string(format!("{} (stream)", base_description), head)
            }
        }
        _ => {
            let value = input.into_value(head);
            if detailed {
                describe_value(value, head, engine_state, call)?
            } else {
                match value {
                    Value::CustomValue { val, .. } => Value::string(val.value_string(), head),
                    _ => Value::string(value.get_type().to_string(), head),
                }
            }
        }
    };

    Ok(description.into_pipeline_data())
}

fn describe_value(
    value: Value,
    head: nu_protocol::Span,
    engine_state: Option<&EngineState>,
    call: &Call,
) -> Result<Value, ShellError> {
    Ok(match value {
        Value::CustomValue { val, internal_span } => Value::record(
            record!(
                "type" => Value::type_literal(Type::Custom(val.value_string()), head),
                "subtype" => run(engine_state,call, val.to_base_value(internal_span)?.into_pipeline_data())?.into_value(head),
            ),
            head,
        ),
        Value::Bool { .. }
        | Value::Int { .. }
        | Value::Float { .. }
        | Value::Filesize { .. }
        | Value::Duration { .. }
        | Value::Date { .. }
        | Value::Range { .. }
        | Value::String { .. }
        | Value::MatchPattern { .. }
        | Value::Nothing { .. } => Value::record(
            record!(
                "type" => Value::type_literal(value.get_type(), head),
            ),
            head,
        ),
        Value::Record { ref val, .. } => {
            let columns = val
                .into_iter()
                .map(|(k, v)| -> Result<(String, Value), ShellError> {
                    Ok((k.to_string(), Value::type_literal(v.get_type(), head)))
                })
                .collect::<Result<_, _>>()?;

            Value::record(
                record!(
                    "type" => Value::type_literal(value.get_type(), head),
                    "lazy" => Value::bool(false, head),
                    "columns" => Value::record(columns, head),
                ),
                head,
            )
        }
        Value::List { ref vals, .. } => Value::record(
            record!(
                "type" => Value::type_literal(value.get_type(), head),
                "length" => Value::int(vals.len() as i64, head),
            ),
            head,
        ),
        Value::Block { val, .. }
        | Value::Closure {
            val: Closure { block_id: val, .. },
            ..
        } => {
            let block = engine_state.map(|engine_state| engine_state.get_block(val));

            if let Some(block) = block {
                let mut record = Record::new();
                record.push("type", Value::type_literal(value.get_type(), head));
                record.push(
                    "signature",
                    Value::record(
                        record!(
                            "name" => Value::string(block.signature.name.clone(), head),
                            "category" => Value::string(block.signature.category.to_string(), head),
                        ),
                        head,
                    ),
                );
                Value::record(record, head)
            } else {
                Value::record(
                    record!(
                        "type" => Value::type_literal(value.get_type(), head),
                    ),
                    head,
                )
            }
        }

        Value::Error { ref error, .. } => Value::record(
            record!(
                "type" => Value::type_literal(value.get_type(), head),
                "subtype" => Value::string(error.to_string(), head),
            ),
            head,
        ),
        Value::Binary { ref val, .. } => Value::record(
            record!(
                "type" => Value::type_literal(value.get_type(), head),
                "length" => Value::int(val.len() as i64, head),
            ),
            head,
        ),
        Value::CellPath { ref val, .. } => Value::record(
            record!(
                "type" => Value::type_literal(value.get_type(), head),
                "length" => Value::int(val.members.len() as i64, head),
            ),
            head,
        ),
        Value::LazyRecord { ref val, .. } => {
            let collect_lazyrecords: bool = call.has_flag("collect-lazyrecords");

            let mut record = Record::new();

            record.push(
                "type",
                if collect_lazyrecords {
                    Value::type_literal(value.get_type(), head)
                } else {
                    Value::type_literal(
                        Type::Record(
                            val.column_names()
                                .into_iter()
                                .map(|c| (c.to_string(), Type::Any))
                                .collect(),
                        ),
                        head,
                    )
                },
            );
            record.push("lazy", Value::bool(true, head));
            record.push("length", Value::int(val.column_names().len() as i64, head));

            if collect_lazyrecords {
                let collected = val.collect()?;
                if let Value::Record { val, .. } = collected {
                    let record_cols = val
                        .into_iter()
                        .map(|(k, v)| -> Result<(String, Value), ShellError> {
                            Ok((k.to_string(), Value::type_literal(v.get_type(), head)))
                        })
                        .collect::<Result<_, _>>()?;
                    record.push("columns", Value::record(record_cols, head));
                }
            }

            Value::record(record, head)
        }
        Value::TypeLiteral { .. } => Value::record(
            record!(
                "type" => Value::type_literal(value.get_type(), head),
            ),
            head,
        ),
    })
}

fn metadata_to_value(metadata: Option<Box<PipelineMetadata>>, head: nu_protocol::Span) -> Value {
    match metadata {
        Some(metadata) => Value::record(
            record!(
                "data_source" => Value::string(format!("{:?}", metadata.data_source), head),
            ),
            head,
        ),
        _ => Value::nothing(head),
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_examples() {
        use super::Describe;
        use crate::test_examples;
        test_examples(Describe {})
    }
}
