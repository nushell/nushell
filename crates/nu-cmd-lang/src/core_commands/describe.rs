use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack, StateWorkingSet};
use nu_protocol::{
    record, Category, Example, IntoPipelineData, PipelineData, Record, ShellError, Signature, Type,
    Value,
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
                example: "{uwu:true, awa:1, name: {last:true, yes:1}} | describe -d",
                result: Some(Value::test_record(record!(
                    "type" => Value::test_string("record"),
                    "fields" => Value::test_record(record!(
                        "uwu" => Value::test_string("bool"),
                        "awa" => Value::test_string("int"),
                        "name" => Value::test_record(record!(
                            "type" => Value::test_string("record"),
                            "fields" => Value::test_record(record!(
                                "last" => Value::test_string("bool"),
                                "yes" => Value::test_string("int"),
                            )),
                        )),
                    )),
                ))),
            },
            /*
            Example {
                description: "Describe a stream of data, collecting it first",
                example: "[1 2 3] | each {|i| $i} | describe",
                result: Some(Value::test_string("list<int> (stream)")),
            },
            Example {
                description: "Describe the input but do not collect streams",
                example: "[1 2 3] | each {|i| $i} | describe --no-collect",
                result: Some(Value::test_string("stream")),
            },
            */
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
    let head = call.head;

    let no_collect: bool = call.has_flag("no-collect");
    let detailed = call.has_flag("detailed");

    let description: Value = match input {
        PipelineData::ExternalStream { .. } => {
            if detailed {
                Value::record(
                    record!(
                        "type" => Value::string("stream", head),
                        "origin" => Value::string("external", head),
                        "raw" => Value::bool(true, head)
                    ),
                    head,
                )
            } else {
                Value::string("raw input", head)
            }
        }
        PipelineData::ListStream(_, _) => {
            // if no_collect {
            //     Value::string("stream".into(), head)
            // } else {
            //     let value = input.into_value(head);
            //     let base_description = match value {
            //         Value::CustomValue { val, .. } => val.value_string(),
            //         _ => value.get_type().to_string(),
            //     };

            //     Value::string(format!("{} (stream)", base_description), head)
            // }
            if detailed {
                Value::record(
                    record!(
                        "type" => Value::string("stream", head),
                        "origin" => Value::string("internal", head),
                        "subtype" => {
                           if no_collect {
                            Value::string("any", head)
                           } else {
                            let value = input.into_value(head);
                            let base_description = match value {
                                Value::CustomValue { val, .. } => val.value_string(),
                                _ => value.get_type().to_string(),
                            };

                            Value::string(base_description, head)
                           }
                        },
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
                match value {
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
                    | Value::Range { .. }
                    | Value::String { .. }
                    | Value::MatchPattern { .. }
                    | Value::Nothing { .. } => Value::record(
                        record!(
                            "type" => Value::string(value.get_type().to_string(), head),
                        ),
                        head,
                    ),
                    Value::Record { val, .. } => {
                        let mut record = Record::new();
                        for i in 0..val.len() {
                            let k = val.cols[i].clone();
                            let v = val.vals[i].clone();

                            record.push(k, {
                                if let Value::Record { val, .. } =
                                    run(engine_state, call, v.into_pipeline_data())?
                                        .into_value(head)
                                {
                                    if let [Value::String { val: k, .. }] = val.vals.as_slice() {
                                        Value::string(k, head)
                                    } else {
                                        Value::record(val, head)
                                    }
                                } else {
                                    unreachable!()
                                }
                            });
                        }

                        Value::record(
                            record!(
                                "type" => Value::string("record", head),
                                "fields" => Value::record(record, head),
                            ),
                            head,
                        )
                    }
                    Value::List { vals, .. } => {
                        let mut fields = vec![];
                        for v in &vals {
                            fields.push(
                                run(engine_state, call, v.clone().into_pipeline_data())?
                                    .into_value(head),
                            );
                        }

                        Value::record(
                            record!(
                                "type" => Value::string("list", head),
                                "length" => Value::int(vals.len() as i64, head),
                                "values" => Value::List { vals: fields, internal_span: head },
                            ),
                            head,
                        )
                    }
                    Value::Block { val, .. } | Value::Closure { val, .. } => {
                        let block = engine_state.map(|engine_state| engine_state.get_block(val));

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
                    Value::LazyRecord { val, .. } => Value::record(
                        record!(
                            "type" => Value::string("lazyrecord", head),
                            "keys" => Value::int(val.column_names().len() as i64, head),
                        ),
                        head,
                    ),
                }
            }
        }
    };

    Ok(description.into_pipeline_data())
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
