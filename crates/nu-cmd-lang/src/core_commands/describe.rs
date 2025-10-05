use nu_engine::command_prelude::*;
use nu_protocol::{
    BlockId, ByteStreamSource, Category, PipelineMetadata, Signature,
    engine::{Closure, StateWorkingSet},
};
use std::any::type_name;
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

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Describe the type of a string",
                example: "'hello' | describe",
                result: Some(Value::test_string("string")),
            },
            Example {
                description: "Describe the type of a record in a detailed way",
                example: "{shell:'true', uwu:true, features: {bugs:false, multiplatform:true, speed: 10}, fib: [1 1 2 3 5 8], on_save: {|x| $'Saving ($x)'}, first_commit: 2019-05-10, my_duration: (4min + 20sec)} | describe -d",
                result: Some(Value::test_record(record!(
                    "type" => Value::test_string("record"),
                    "detailed_type" => Value::test_string("record<shell: string, uwu: bool, features: record<bugs: bool, multiplatform: bool, speed: int>, fib: list<int>, on_save: closure, first_commit: datetime, my_duration: duration>"),
                    "columns" => Value::test_record(record!(
                        "shell" => Value::test_record(record!(
                            "type" => Value::test_string("string"),
                            "detailed_type" => Value::test_string("string"),
                            "rust_type" => Value::test_string("&alloc::string::String"),
                            "value" => Value::test_string("true"),
                        )),
                        "uwu" => Value::test_record(record!(
                            "type" => Value::test_string("bool"),
                            "detailed_type" => Value::test_string("bool"),
                            "rust_type" => Value::test_string("bool"),
                            "value" => Value::test_bool(true),
                        )),
                        "features" => Value::test_record(record!(
                            "type" => Value::test_string("record"),
                            "detailed_type" => Value::test_string("record<bugs: bool, multiplatform: bool, speed: int>"),
                            "columns" => Value::test_record(record!(
                                "bugs" => Value::test_record(record!(
                                    "type" => Value::test_string("bool"),
                                    "detailed_type" => Value::test_string("bool"),
                                    "rust_type" => Value::test_string("bool"),
                                    "value" => Value::test_bool(false),
                                )),
                                "multiplatform" => Value::test_record(record!(
                                    "type" => Value::test_string("bool"),
                                    "detailed_type" => Value::test_string("bool"),
                                    "rust_type" => Value::test_string("bool"),
                                    "value" => Value::test_bool(true),
                                )),
                                "speed" => Value::test_record(record!(
                                    "type" => Value::test_string("int"),
                                    "detailed_type" => Value::test_string("int"),
                                    "rust_type" => Value::test_string("i64"),
                                    "value" => Value::test_int(10),
                                )),
                            )),
                            "rust_type" => Value::test_string("&nu_utils::shared_cow::SharedCow<nu_protocol::value::record::Record>"),
                        )),
                        "fib" => Value::test_record(record!(
                            "type" => Value::test_string("list"),
                            "detailed_type" => Value::test_string("list<int>"),
                            "length" => Value::test_int(6),
                            "rust_type" => Value::test_string("&mut alloc::vec::Vec<nu_protocol::value::Value>"),
                            "value" => Value::test_list(vec![
                                Value::test_record(record!(
                                    "type" => Value::test_string("int"),
                                    "detailed_type" => Value::test_string("int"),
                                    "rust_type" => Value::test_string("i64"),
                                    "value" => Value::test_int(1),
                                )),
                                Value::test_record(record!(
                                    "type" => Value::test_string("int"),
                                    "detailed_type" => Value::test_string("int"),
                                    "rust_type" => Value::test_string("i64"),
                                    "value" => Value::test_int(1),
                                )),
                                Value::test_record(record!(
                                    "type" => Value::test_string("int"),
                                    "detailed_type" => Value::test_string("int"),
                                    "rust_type" => Value::test_string("i64"),
                                    "value" => Value::test_int(2),
                                )),
                                Value::test_record(record!(
                                    "type" => Value::test_string("int"),
                                    "detailed_type" => Value::test_string("int"),
                                    "rust_type" => Value::test_string("i64"),
                                    "value" => Value::test_int(3),
                                )),
                                Value::test_record(record!(
                                    "type" => Value::test_string("int"),
                                    "detailed_type" => Value::test_string("int"),
                                    "rust_type" => Value::test_string("i64"),
                                    "value" => Value::test_int(5),
                                )),
                                Value::test_record(record!(
                                    "type" => Value::test_string("int"),
                                    "detailed_type" => Value::test_string("int"),
                                    "rust_type" => Value::test_string("i64"),
                                    "value" => Value::test_int(8),
                                ))]
                        ),
                        )),
                        "on_save" => Value::test_record(record!(
                            "type" => Value::test_string("closure"),
                            "detailed_type" => Value::test_string("closure"),
                            "rust_type" => Value::test_string("&alloc::boxed::Box<nu_protocol::engine::closure::Closure>"),
                            "value" => Value::test_closure(Closure {
                                block_id: BlockId::new(1),
                                captures: vec![],
                            }),
                            "signature" => Value::test_record(record!(
                                "name" => Value::test_string(""),
                                "category" => Value::test_string("default"),
                            )),
                        )),
                        "first_commit" => Value::test_record(record!(
                            "type" => Value::test_string("datetime"),
                            "detailed_type" => Value::test_string("datetime"),
                            "rust_type" => Value::test_string("chrono::datetime::DateTime<chrono::offset::fixed::FixedOffset>"),
                            "value" => Value::test_date("2019-05-10 00:00:00Z".parse().unwrap_or_default()),
                        )),
                        "my_duration" => Value::test_record(record!(
                            "type" => Value::test_string("duration"),
                            "detailed_type" => Value::test_string("duration"),
                            "rust_type" => Value::test_string("i64"),
                            "value" => Value::test_duration(260_000_000_000),
                        ))
                    )),
                    "rust_type" => Value::test_string("&nu_utils::shared_cow::SharedCow<nu_protocol::value::record::Record>"),
                ))),
            },
            Example {
                description: "Describe the type of a stream with detailed information",
                example: "[1 2 3] | each {|i| echo $i} | describe -d",
                result: None, // Give "Running external commands not supported" error
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
                result: None, // Give "Running external commands not supported" error
                              // result: Some(Value::test_string("list<int> (stream)")),
            },
            Example {
                description: "Describe the input but do not collect streams",
                example: "[1 2 3] | each {|i| echo $i} | describe --no-collect",
                result: None, // Give "Running external commands not supported" error
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
                        "type" => Value::string("bytestream", head),
                        "detailed_type" => Value::string(type_, head),
                        "rust_type" => Value::string(type_of(&stream), head),
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
            let type_ = type_of(&stream);
            if options.detailed {
                let subtype = if options.no_collect {
                    Value::string("any", head)
                } else {
                    describe_value(stream.into_debug_value(), head, engine_state)
                };
                Value::record(
                    record! {
                        "type" => Value::string("stream", head),
                        "detailed_type" => Value::string("list stream", head),
                        "rust_type" => Value::string(type_, head),
                        "origin" => Value::string("nushell", head),
                        "subtype" => subtype,
                        "metadata" => metadata_to_value(metadata, head),
                    },
                    head,
                )
            } else if options.no_collect {
                Value::string("stream", head)
            } else {
                let value = stream.into_debug_value();
                let base_description = value.get_type().to_string();
                Value::string(format!("{base_description} (stream)"), head)
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
    Record(Record),
}

impl Description {
    fn into_value(self, span: Span) -> Value {
        match self {
            Description::Record(record) => Value::record(record, span),
        }
    }
}

fn describe_value(value: Value, head: Span, engine_state: Option<&EngineState>) -> Value {
    let Description::Record(record) = describe_value_inner(value, head, engine_state);
    Value::record(record, head)
}

fn type_of<T>(_: &T) -> String {
    type_name::<T>().to_string()
}

fn describe_value_inner(
    mut value: Value,
    head: Span,
    engine_state: Option<&EngineState>,
) -> Description {
    let value_type = value.get_type().to_string();
    match value {
        Value::Bool { val, .. } => Description::Record(record! {
            "type" => Value::string("bool", head),
            "detailed_type" => Value::string(value_type, head),
            "rust_type" => Value::string(type_of(&val), head),
            "value" => value,
        }),
        Value::Int { val, .. } => Description::Record(record! {
            "type" => Value::string("int", head),
            "detailed_type" => Value::string(value_type, head),
            "rust_type" => Value::string(type_of(&val), head),
            "value" => value,
        }),
        Value::Float { val, .. } => Description::Record(record! {
            "type" => Value::string("float", head),
            "detailed_type" => Value::string(value_type, head),
            "rust_type" => Value::string(type_of(&val), head),
            "value" => value,
        }),
        Value::Filesize { val, .. } => Description::Record(record! {
            "type" => Value::string("filesize", head),
            "detailed_type" => Value::string(value_type, head),
            "rust_type" => Value::string(type_of(&val), head),
            "value" => value,
        }),
        Value::Duration { val, .. } => Description::Record(record! {
            "type" => Value::string("duration", head),
            "detailed_type" => Value::string(value_type, head),
            "rust_type" => Value::string(type_of(&val), head),
            "value" => value,
        }),
        Value::Date { val, .. } => Description::Record(record! {
            "type" => Value::string("datetime", head),
            "detailed_type" => Value::string(value_type, head),
            "rust_type" => Value::string(type_of(&val), head),
            "value" => value,
        }),
        Value::Range { ref val, .. } => Description::Record(record! {
            "type" => Value::string("range", head),
            "detailed_type" => Value::string(value_type, head),
            "rust_type" => Value::string(type_of(&val), head),
            "value" => value,
        }),
        Value::String { ref val, .. } => Description::Record(record! {
            "type" => Value::string("string", head),
            "detailed_type" => Value::string(value_type, head),
            "rust_type" => Value::string(type_of(&val), head),
            "value" => value,
        }),
        Value::Glob { ref val, .. } => Description::Record(record! {
            "type" => Value::string("glob", head),
            "detailed_type" => Value::string(value_type, head),
            "rust_type" => Value::string(type_of(&val), head),
            "value" => value,
        }),
        Value::Nothing { .. } => Description::Record(record! {
            "type" => Value::string("nothing", head),
            "detailed_type" => Value::string(value_type, head),
            "rust_type" => Value::string("", head),
            "value" => value,
        }),
        Value::Record { ref val, .. } => {
            let mut columns = val.clone().into_owned();
            for (_, val) in &mut columns {
                *val =
                    describe_value_inner(std::mem::take(val), head, engine_state).into_value(head);
            }

            Description::Record(record! {
                "type" => Value::string("record", head),
                "detailed_type" => Value::string(value_type, head),
                "columns" => Value::record(columns.clone(), head),
                "rust_type" => Value::string(type_of(&val), head),
            })
        }
        Value::List { ref mut vals, .. } => {
            for val in &mut *vals {
                *val =
                    describe_value_inner(std::mem::take(val), head, engine_state).into_value(head);
            }

            Description::Record(record! {
                "type" => Value::string("list", head),
                "detailed_type" => Value::string(value_type, head),
                "length" => Value::int(vals.len() as i64, head),
                "rust_type" => Value::string(type_of(&vals), head),
                "value" => value,
            })
        }
        Value::Closure { ref val, .. } => {
            let block = engine_state.map(|engine_state| engine_state.get_block(val.block_id));

            let mut record = record! {
                "type" => Value::string("closure", head),
                "detailed_type" => Value::string(value_type, head),
                "rust_type" => Value::string(type_of(&val), head),
                "value" => value,
            };
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
        Value::Error { ref error, .. } => Description::Record(record! {
            "type" => Value::string("error", head),
            "detailed_type" => Value::string(value_type, head),
            "subtype" => Value::string(error.to_string(), head),
            "rust_type" => Value::string(type_of(&error), head),
            "value" => value,
        }),
        Value::Binary { ref val, .. } => Description::Record(record! {
            "type" => Value::string("binary", head),
            "detailed_type" => Value::string(value_type, head),
            "length" => Value::int(val.len() as i64, head),
            "rust_type" => Value::string(type_of(&val), head),
            "value" => value,
        }),
        Value::CellPath { ref val, .. } => Description::Record(record! {
            "type" => Value::string("cell-path", head),
            "detailed_type" => Value::string(value_type, head),
            "length" => Value::int(val.members.len() as i64, head),
            "rust_type" => Value::string(type_of(&val), head),
            "value" => value
        }),
        Value::Custom { ref val, .. } => Description::Record(record! {
            "type" => Value::string("custom", head),
            "detailed_type" => Value::string(value_type, head),
            "subtype" => Value::string(val.type_name(), head),
            "rust_type" => Value::string(type_of(&val), head),
            "value" =>
                match val.to_base_value(head) {
                    Ok(base_value) => base_value,
                    Err(err) => Value::error(err, head),
                }
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
