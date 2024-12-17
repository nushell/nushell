use nu_engine::command_prelude::*;
use nu_protocol::{engine::StateWorkingSet, ByteStreamSource, PipelineMetadata};

#[derive(Clone)]
pub struct Describe;

impl Command for Describe {
    fn name(&self) -> &str {
        "describe"
    }

    fn description(&self) -> &str {
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
                    "{shell:'true', uwu:true, features: {bugs:false, multiplatform:true, speed: 10}, fib: [1 1 2 3 5 8], on_save: {|x| $'Saving ($x)'}, first_commit: 2019-05-10, my_duration: (4min + 20sec)} | describe -d",
                result: Some(Value::test_record(record!(
                    "type" => Value::test_string("record"),
                    "columns" => Value::test_record(record!(
                        "shell" => Value::test_string("string"),
                        "uwu" => Value::test_string("bool"),
                        "features" => Value::test_record(record!(
                            "type" => Value::test_string("record"),
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
}

fn run(
    engine_state: Option<&EngineState>,
    call: &Call,
    input: PipelineData,
    options: Options,
) -> Result<PipelineData, ShellError> {
    let head = call.head;
    let metadata = input.metadata();

    let description = match input {
        PipelineData::ByteStream(stream, ..) => {
            let type_ = stream.type_().describe();

            let description = if options.detailed {
                let origin = match stream.source() {
                    ByteStreamSource::Read(_) => "unknown",
                    ByteStreamSource::File(_) => "file",
                    #[cfg(feature = "os")]
                    ByteStreamSource::Child(_) => "external",
                };

                Value::record(
                    record! {
                        "type" => Value::string(type_, head),
                        "origin" => Value::string(origin, head),
                        "metadata" => metadata_to_value(metadata, head),
                    },
                    head,
                )
            } else {
                Value::string(type_, head)
            };

            if !options.no_collect {
                stream.drain()?;
            }

            description
        }
        PipelineData::ListStream(stream, ..) => {
            if options.detailed {
                let subtype = if options.no_collect {
                    Value::string("any", head)
                } else {
                    describe_value(stream.into_value(), head, engine_state)
                };
                Value::record(
                    record! {
                        "type" => Value::string("stream", head),
                        "origin" => Value::string("nushell", head),
                        "subtype" => subtype,
                        "metadata" => metadata_to_value(metadata, head),
                    },
                    head,
                )
            } else if options.no_collect {
                Value::string("stream", head)
            } else {
                let value = stream.into_value();
                let base_description = value.get_type().to_string();
                Value::string(format!("{} (stream)", base_description), head)
            }
        }
        PipelineData::Value(value, ..) => {
            if !options.detailed {
                Value::string(value.get_type().to_string(), head)
            } else {
                describe_value(value, head, engine_state)
            }
        }
        PipelineData::Empty => Value::string(Type::Nothing.to_string(), head),
    };

    Ok(description.into_pipeline_data())
}

enum Description {
    String(String),
    Record(Record),
}

impl Description {
    fn into_value(self, span: Span) -> Value {
        match self {
            Description::String(ty) => Value::string(ty, span),
            Description::Record(record) => Value::record(record, span),
        }
    }
}

fn describe_value(value: Value, head: Span, engine_state: Option<&EngineState>) -> Value {
    let record = match describe_value_inner(value, head, engine_state) {
        Description::String(ty) => record! { "type" => Value::string(ty, head) },
        Description::Record(record) => record,
    };
    Value::record(record, head)
}

fn describe_value_inner(
    value: Value,
    head: Span,
    engine_state: Option<&EngineState>,
) -> Description {
    match value {
        Value::Bool { .. }
        | Value::Int { .. }
        | Value::Float { .. }
        | Value::Filesize { .. }
        | Value::Duration { .. }
        | Value::Date { .. }
        | Value::Range { .. }
        | Value::String { .. }
        | Value::Glob { .. }
        | Value::Nothing { .. } => Description::String(value.get_type().to_string()),
        Value::Record { val, .. } => {
            let mut columns = val.into_owned();
            for (_, val) in &mut columns {
                *val =
                    describe_value_inner(std::mem::take(val), head, engine_state).into_value(head);
            }

            Description::Record(record! {
                "type" => Value::string("record", head),
                "columns" => Value::record(columns, head),
            })
        }
        Value::List { mut vals, .. } => {
            for val in &mut vals {
                *val =
                    describe_value_inner(std::mem::take(val), head, engine_state).into_value(head);
            }

            Description::Record(record! {
                "type" => Value::string("list", head),
                "length" => Value::int(vals.len() as i64, head),
                "values" => Value::list(vals, head),
            })
        }
        Value::Closure { val, .. } => {
            let block = engine_state.map(|engine_state| engine_state.get_block(val.block_id));

            let mut record = record! { "type" => Value::string("closure", head) };
            if let Some(block) = block {
                record.push(
                    "signature",
                    Value::record(
                        record! {
                            "name" => Value::string(block.signature.name.clone(), head),
                            "category" => Value::string(block.signature.category.to_string(), head),
                        },
                        head,
                    ),
                );
            }
            Description::Record(record)
        }
        Value::Error { error, .. } => Description::Record(record! {
            "type" => Value::string("error", head),
            "subtype" => Value::string(error.to_string(), head),
        }),
        Value::Binary { val, .. } => Description::Record(record! {
            "type" => Value::string("binary", head),
            "length" => Value::int(val.len() as i64, head),
        }),
        Value::CellPath { val, .. } => Description::Record(record! {
            "type" => Value::string("cell-path", head),
            "length" => Value::int(val.members.len() as i64, head),
        }),
        Value::Custom { val, .. } => Description::Record(record! {
            "type" => Value::string("custom", head),
            "subtype" => Value::string(val.type_name(), head),
        }),
    }
}

fn metadata_to_value(metadata: Option<PipelineMetadata>, head: Span) -> Value {
    if let Some(metadata) = metadata {
        let data_source = Value::string(format!("{:?}", metadata.data_source), head);
        Value::record(record! { "data_source" => data_source }, head)
    } else {
        Value::nothing(head)
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
