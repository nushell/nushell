use nu_engine::command_prelude::*;
use nu_protocol::{PipelineMetadata, casing::WrapCased};

#[derive(Clone)]
pub struct Peek;

impl Command for Peek {
    fn name(&self) -> &str {
        "peek"
    }

    fn description(&self) -> &str {
        "Peek the first <n> elements of a stream and store them in the metadata."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["stream", "inspect"]
    }

    fn signature(&self) -> Signature {
        Signature::build("peek")
            .input_output_types(vec![
                (
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::Any)),
                ),
                (Type::table(), Type::table()),
                (Type::Any, Type::Any),
            ])
            .optional(
                "n",
                SyntaxShape::Int,
                "Number of elements to peek, if the input is a stream or list.",
            )
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let n: Option<usize> = call.opt(engine_state, stack, 0)?;

        match input {
            PipelineData::Empty => {
                let metadata = add_peek_metadata(None, "empty", None, call.head);
                Ok(Value::nothing(call.head).into_pipeline_data_with_metadata(metadata))
            }
            PipelineData::Value(val, metadata) => match &val {
                Value::List { vals, .. } => {
                    let peeked = n.map(|n| {
                        vals.iter()
                            .take(n)
                            .cloned()
                            .collect::<Vec<_>>()
                            .into_value(call.head)
                    });
                    let metadata = add_peek_metadata(metadata, "list", peeked, call.head);
                    Ok(PipelineData::value(val, metadata))
                }
                _ => {
                    let metadata = add_peek_metadata(metadata, "value", None, call.head);
                    Ok(PipelineData::value(val, metadata))
                }
            },
            PipelineData::ListStream(stream, metadata) => {
                let mut elems = None;
                let stream = match n {
                    Some(n) => stream.modify(|mut it| {
                        let collect = it.as_mut().take(n).collect::<Vec<_>>();
                        elems = Some(collect.clone());
                        collect.into_iter().chain(it)
                    }),
                    None => stream,
                };

                let metadata = add_peek_metadata(
                    metadata,
                    "list (stream)",
                    elems.map(|x| x.into_value(call.head)),
                    call.head,
                );

                Ok(PipelineData::list_stream(stream, metadata))
            }
            PipelineData::ByteStream(byte_stream, pipeline_metadata) => {
                let metadata = add_peek_metadata(
                    pipeline_metadata,
                    byte_stream.type_().describe(),
                    None,
                    call.head,
                );
                Ok(PipelineData::byte_stream(byte_stream, metadata))
            }
        }
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Peek the first 2 elements of a stream.",
                example: r#"seq 1 5 | peek 2 | metadata | $in.peek"#,
                result: Some(Value::test_record(record! {
                    "type" => "list (stream)".into_value(Span::test_data()),
                    "value" => [1, 2].into_value(Span::test_data()),
                })),
            },
            Example {
                description: "Lists can also be peeked.",
                example: r#"[1, 2, 3] | peek 1 | metadata | $in.peek"#,
                result: Some(Value::test_record(record! {
                    "type" => "list".into_value(Span::test_data()),
                    "value" => [1].into_value(Span::test_data()),
                })),
            },
            Example {
                description: "Peeking non-list values won't return any values.",
                example: r#"'hello' | peek 1 | metadata | $in.peek"#,
                result: Some(Value::test_record(record! {
                    "type" => "value".into_value(Span::test_data())
                })),
            },
            Example {
                description: "Peeking non-list streams (text streams, binary streams, external byte streams) won't return any values.",
                example: r#"[0x[11] 0x[13 15]] | bytes collect | peek | metadata | $in.peek"#,
                result: Some(Value::test_record(record! {
                    "type" => "binary (stream)".into_value(Span::test_data())
                })),
            },
        ]
    }
}

fn add_peek_metadata(
    mut metadata: Option<PipelineMetadata>,
    r#type: &str,
    value: Option<Value>,
    span: Span,
) -> Option<PipelineMetadata> {
    let mut record = Record::new();
    let record_handle = record.as_mut().case_sensitive();

    record_handle.insert("type", r#type.into_value(span));
    if let Some(value) = value {
        record_handle.insert("value", value);
    }

    metadata
        .get_or_insert_default()
        .custom
        .insert("peek", record.into_value(span));

    metadata
}
