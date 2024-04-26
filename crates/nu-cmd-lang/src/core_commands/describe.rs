use nu_engine::command_prelude::*;
use nu_protocol::{engine::StateWorkingSet, PipelineMetadata};

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
            .input_output_types(vec![(Type::Any, Type::Any)])
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
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let options = Options {
            no_collect: call.has_flag(engine_state, stack, "no-collect")?,
            detailed: call.has_flag(engine_state, stack, "detailed")?,
            collect_lazyrecords: call.has_flag(engine_state, stack, "collect-lazyrecords")?,
        };
        run(Some(engine_state), call, input, options)
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let options = Options {
            no_collect: call.has_flag_const(working_set, "no-collect")?,
            detailed: call.has_flag_const(working_set, "detailed")?,
            collect_lazyrecords: call.has_flag_const(working_set, "collect-lazyrecords")?,
        };
        run(None, call, input, options)
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

#[derive(Clone, Copy)]
struct Options {
    no_collect: bool,
    detailed: bool,
    collect_lazyrecords: bool,
}

fn run(
    engine_state: Option<&EngineState>,
    call: &Call,
    input: PipelineData,
    options: Options,
) -> Result<PipelineData, ShellError> {
    let metadata = input.metadata().clone().map(Box::new);
    let head = call.head;

    let description: Value = match input {
        PipelineData::ExternalStream {
            ref stdout,
            ref stderr,
            ref exit_code,
            ..
        } => {
            if options.detailed {
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
            if options.detailed {
                Value::record(
                    record!(
                        "type" => Value::string("stream", head),
                        "origin" => Value::string("nushell", head),
                        "subtype" => {
                           if options.no_collect {
                            Value::string("any", head)
                           } else {
                            describe_value(input.into_value(head), head, engine_state, options)?
                           }
                        },
                        "metadata" => metadata_to_value(metadata, head),
                    ),
                    head,
                )
            } else if options.no_collect {
                Value::string("stream", head)
            } else {
                let value = input.into_value(head);
                let base_description = value.get_type().to_string();

                Value::string(format!("{} (stream)", base_description), head)
            }
        }
        _ => {
            let value = input.into_value(head);
            if !options.detailed {
                Value::string(value.get_type().to_string(), head)
            } else {
                describe_value(value, head, engine_state, options)?
            }
        }
    };

    Ok(description.into_pipeline_data())
}

fn compact_primitive_description(mut value: Value) -> Value {
    if let Value::Record { ref mut val, .. } = value {
        if val.len() != 1 {
            return value;
        }
        if let Some(type_name) = val.to_mut().get_mut("type") {
            return std::mem::take(type_name);
        }
    }
    value
}

fn describe_value(
    value: Value,
    head: nu_protocol::Span,
    engine_state: Option<&EngineState>,
    options: Options,
) -> Result<Value, ShellError> {
    Ok(match value {
        Value::Custom { val, .. } => Value::record(
            record!(
                "type" => Value::string("custom", head),
                "subtype" => Value::string(val.type_name(), head),
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
        | Value::Glob { .. }
        | Value::Nothing { .. } => Value::record(
            record!(
                "type" => Value::string(value.get_type().to_string(), head),
            ),
            head,
        ),
        Value::Record { val, .. } => {
            let mut val = val.into_owned();
            for (_k, v) in val.iter_mut() {
                *v = compact_primitive_description(describe_value(
                    std::mem::take(v),
                    head,
                    engine_state,
                    options,
                )?);
            }

            Value::record(
                record!(
                    "type" => Value::string("record", head),
                    "lazy" => Value::bool(false, head),
                    "columns" => Value::record(val, head),
                ),
                head,
            )
        }
        Value::List { vals, .. } => Value::record(
            record!(
                "type" => Value::string("list", head),
                "length" => Value::int(vals.len() as i64, head),
                "values" => Value::list(vals.into_iter().map(|v|
                    Ok(compact_primitive_description(
                        describe_value(v, head, engine_state, options)?
                    ))
                )
                .collect::<Result<Vec<Value>, ShellError>>()?, head),
            ),
            head,
        ),
        Value::Closure { val, .. } => {
            let block = engine_state.map(|engine_state| engine_state.get_block(val.block_id));

            if let Some(block) = block {
                let mut record = Record::new();
                record.push("type", Value::string("closure", head));
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
                        "type" => Value::string("closure", head),
                    ),
                    head,
                )
            }
        }

        Value::Error { error, .. } => Value::record(
            record!(
                "type" => Value::string("error", head),
                "subtype" => Value::string(error.to_string(), head),
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
                "length" => Value::int(val.members.len() as i64, head),
            ),
            head,
        ),
        Value::LazyRecord { val, .. } => {
            let mut record = Record::new();

            record.push("type", Value::string("record", head));
            record.push("lazy", Value::bool(true, head));

            if options.collect_lazyrecords {
                let collected = val.collect()?;
                if let Value::Record { val, .. } =
                    describe_value(collected, head, engine_state, options)?
                {
                    let mut val = Record::clone(&val);

                    for (_k, v) in val.iter_mut() {
                        *v = compact_primitive_description(describe_value(
                            std::mem::take(v),
                            head,
                            engine_state,
                            options,
                        )?);
                    }

                    record.push("length", Value::int(val.len() as i64, head));
                    record.push("columns", Value::record(val, head));
                } else {
                    let cols = val.column_names();
                    record.push("length", Value::int(cols.len() as i64, head));
                }
            } else {
                let cols = val.column_names();
                record.push("length", Value::int(cols.len() as i64, head));
            }

            Value::record(record, head)
        }
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
