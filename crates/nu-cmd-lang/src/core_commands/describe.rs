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
                (Type::Any, Type::String),
                (Type::Any, Type::Record(vec![])),
            ])
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
                    "type" => Value::test_string("record"),
                    "lazy" => Value::test_bool(false),
                    "columns" => Value::test_record(record!(
                        "shell" => Value::test_string("string"),
                        "uwu" => Value::test_string("bool"),
                        "features" => Value::test_record(record!(
                            "type" => Value::test_string("record"),
                            "lazy" => Value::test_bool(false),
                            "columns" => Value::test_record(record!(
                                "bugs" => Value::test_string("bool"),
                                "multiplatform" => Value::test_string("bool"),
                                "speed" => Value::test_string("int"),
                            )),
                        )),
                        "fib" => Value::test_record(record!(
                            "type" => Value::test_string("list"),
                            "length" => Value::test_int(6),
                            "values" => Value::test_list(vec![
                                Value::test_string("int"),
                                Value::test_string("int"),
                                Value::test_string("int"),
                                Value::test_string("int"),
                                Value::test_string("int"),
                                Value::test_string("int"),
                           ]),
                        )),
                        "on_save" => Value::test_record(record!(
                            "type" => Value::test_string("closure"),
                            "signature" => Value::test_record(record!(
                                "name" => Value::test_string(""),
                                "category" => Value::test_string("default"),
                            )),
                        )),
                        "first_commit" => Value::test_string("date"),
                        "my_duration" => Value::test_string("duration"),
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
                        "type" => Value::string("stream", head),
                        "origin" => Value::string("external", head),
                        "stdout" => match stdout {
                            Some(_) => Value::record(
                                    record!(
                                        "type" => Value::string("stream", head),
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
                                        "type" => Value::string("stream", head),
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
                                        "type" => Value::string("stream", head),
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
                Value::record(
                    record!(
                        "type" => Value::string("stream", head),
                        "origin" => Value::string("nushell", head),
                        "subtype" => {
                           if no_collect {
                            Value::string("any", head)
                           } else {
                            describe_value(input.into_value(head), head, engine_state, call)?
                           }
                        },
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
            if !detailed {
                match value {
                    Value::CustomValue { val, .. } => Value::string(val.value_string(), head),
                    _ => Value::string(value.get_type().to_string(), head),
                }
            } else {
                describe_value(value, head, engine_state, call)?
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
                "type" => Value::string("custom", head),
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
        | Value::String { .. }
        | Value::MatchPattern { .. }
        | Value::Nothing { .. } => Value::record(
            record!(
                "type" => Value::string(value.get_type().to_string(), head),
            ),
            head,
        ),
        Value::Range { val, .. } => Value::record(
            record!(
                "type" => Value::string("range", head),
                "from" => val.from,
                "to" => val.to,
                "increment" => val.incr,
                "inclusion" => Value::record(record!(
                    "left" => Value::bool(match val.inclusion {
                        nu_protocol::ast::RangeInclusion::Inclusive => true,
                        nu_protocol::ast::RangeInclusion::RightExclusive => true,
                    }, head),
                    "right" => Value::bool(match val.inclusion {
                        nu_protocol::ast::RangeInclusion::Inclusive => true,
                        nu_protocol::ast::RangeInclusion::RightExclusive => false,
                    }, head),
                ), head)
            ),
            head,
        ),
        Value::Record { val, .. } => describe_record(val, head, engine_state, call, false)?,
        Value::List { vals, .. } => Value::record(
            record!(
                "type" => Value::string("list", head),
                "length" => Value::int(vals.len() as i64, head),
                "values" => Value::list(vals.iter().map(|v|
                    match describe_value(v.clone(), head, engine_state, call) {
                        Ok(Value::Record {val, ..}) => if val.cols.as_slice() == ["type"] {Ok(val.vals[0].clone())} else {Ok(Value::record(val, head))},
                        x => x
                    }
                ).collect::<Result<Vec<_>, _>>()?, head),
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
                record.push("type", Value::string(value.get_type().to_string(), head));
                record.push(
                    "signature",
                    Value::record(
                        record!(
                            "name" => Value::string(block.signature.name.clone(), head),
                            "usage" => Value::string(block.signature.usage.clone(), head),
                            "category" => Value::string(block.signature.category.to_string(), head),
                            "required_positionals" => Value::list(block.signature.required_positional.iter().map(|x| 
                                Value::record(record!(
                                    "name" => Value::string(x.name.clone(), head),
                                    "type" => Value::string(x.shape.to_string(), head),
                                    "default_value" => x.default_value.clone().unwrap_or(Value::nothing(head)),
                                ), head)
                             ).collect::<Vec<_>>(), head),
                             "optional_positionals" => Value::list(block.signature.optional_positional.iter().map(|x| 
                                Value::record(record!(
                                    "name" => Value::string(x.name.clone(), head),
                                    "type" => Value::string(x.shape.to_string(), head),
                                    "default_value" => x.default_value.clone().unwrap_or(Value::nothing(head)),
                                ), head)
                             ).collect::<Vec<_>>(), head),
                             "rest_positional" => block.signature.rest_positional.as_ref().map(|x| 
                                Value::record(record!(
                                    "name" => Value::string(x.name.clone(), head),
                                    "type" => Value::string(x.shape.to_string(), head),
                                    "default_value" => x.default_value.clone().unwrap_or(Value::nothing(head)),
                                ), head)
                             ).unwrap_or(Value::nothing(head)),
                             "named" => Value::list(block.signature.named.iter().map(|x| 
                                Value::record(record!(
                                    "long" => Value::string(x.long.clone(), head),
                                    "short" => x.short.as_ref().map(|x| Value::string(x.to_string(), head)).unwrap_or(Value::nothing(head)),
                                    "arg" => x.arg.as_ref().map(|x| Value::string(x.to_string(), head)).unwrap_or(Value::nothing(head)),
                                    "required" => Value::bool(x.required, head),
                                    "desc" => Value::string(x.desc.clone(), head),
                                    "default_value" => x.default_value.clone().unwrap_or(Value::nothing(head)),
                                ), head)
                             ).collect::<Vec<_>>(), head),
                             "input_output_types" => Value::list(block.signature.input_output_types.iter().map(|x| 
                                Value::record(record!(
                                    "input" => Value::string(x.0.to_string(), head),
                                    "output" => Value::string(x.1.to_string(), head),
                                ), head)
                             ).collect::<Vec<_>>(), head),
                             "is_filter" => Value::bool(block.signature.is_filter, head),
                             "creates_scope" => Value::bool(block.signature.creates_scope, head),
                             "allows_unknown_args" => Value::bool(block.signature.allows_unknown_args, head),
                        ),
                        head,
                    ),
                );
                Value::record(record, head)
            } else {
                Value::record(
                    record!(
                        "type" => Value::string("closure", head),
                    ),
                    head,
                )
            }
        }
        Value::Error { error, .. } => Value::record(
            record!(
                "type" => Value::string("error", head),
                "subtype" => Value::string(error.as_ref().as_ref(), head),
            ),
            head,
        ),
        Value::Binary { val, .. } => Value::record(
            record!(
                "type" => Value::string("binary", head),
                "length" => Value::int(val.len() as i64, head),
            ),
            head,
        ),
        Value::CellPath { val, .. } => Value::record(
            record!(
                "type" => Value::string("cellpath", head),
                "members" => Value::list(val.members.iter().map(|x|
                    match x {
                        nu_protocol::ast::PathMember::Int {val, ..} => Value::record(record!(
                            "type" => Value::string("int", head),
                            "value" => Value::int(*val as i64, head),
                        ), head),
                        nu_protocol::ast::PathMember::String {val, ..} => Value::record(record!(
                            "type" => Value::string("string", head),
                            "value" => Value::string(val.clone(), head),
                        ), head),
                    }
                ).collect::<Vec<_>>(), head),
            ),
            head,
        ),
        Value::LazyRecord { val, .. } => {
            let collect_lazyrecords: bool = call.has_flag("collect-lazyrecords");

            if collect_lazyrecords {
                let collected = val.collect()?;
                if let Value::Record { val, .. } = collected {
                    describe_record(val, head, engine_state, call, true)?
                } else {
                    return Err(ShellError::CantConvert {
                        from_type: collected.get_type().to_string(),
                        to_type: "record".to_string(),
                        span: head,
                        help: None,
                    });
                }
            } else {
                Value::record(
                    record!(
                        "type" => Value::string("record", head),
                        "lazy" => Value::bool(true, head),
                        "length" => Value::int(val.column_names().len() as i64, head)
                    ),
                    head,
                )
            }
        }
    })
}

fn describe_record(
    val: Record,
    head: nu_protocol::Span,
    engine_state: Option<&EngineState>,
    call: &Call,
    is_lazy: bool,
) -> Result<Value, ShellError> {
    let mut record = Record::new();
    for i in 0..val.len() {
        let k = val.cols[i].clone();
        let v = val.vals[i].clone();

        record.push(k, {
            if let Value::Record { val, .. } = describe_value(v.clone(), head, engine_state, call)?
            {
                if let [Value::String { val: k, .. }] = val.vals.as_slice() {
                    Value::string(k, head)
                } else {
                    Value::record(val, head)
                }
            } else {
                describe_value(v, head, engine_state, call)?
            }
        });
    }
    Ok(Value::record(
        record!(
            "type" => Value::string("record", head),
            "lazy" => Value::bool(is_lazy, head),
            "columns" => Value::record(record, head),
        ),
        head,
    ))
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
