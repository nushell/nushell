use chrono::Datelike;
use chrono_humanize::HumanTime;
use nu_engine::command_prelude::*;
use nu_protocol::{ByteStream, PipelineMetadata, format_duration, shell_error::io::IoError};
use nu_utils::ObviousFloat;
use std::io::Write;

const LINE_ENDING: &str = if cfg!(target_os = "windows") {
    "\r\n"
} else {
    "\n"
};

#[derive(Clone)]
pub struct ToText;

impl Command for ToText {
    fn name(&self) -> &str {
        "to text"
    }

    fn signature(&self) -> Signature {
        Signature::build("to text")
            .input_output_types(vec![(Type::Any, Type::String)])
            .switch(
                "no-newline",
                "Do not append a newline to the end of the text",
                Some('n'),
            )
            .switch(
                "serialize",
                "serialize nushell types that cannot be deserialized",
                Some('s'),
            )
            .category(Category::Formats)
    }

    fn description(&self) -> &str {
        "Converts data into simple text."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let no_newline = call.has_flag(engine_state, stack, "no-newline")?;
        let serialize_types = call.has_flag(engine_state, stack, "serialize")?;
        let input = input.try_expand_range()?;

        match input {
            PipelineData::Empty => Ok(Value::string(String::new(), head)
                .into_pipeline_data_with_metadata(update_metadata(None))),
            PipelineData::Value(value, ..) => {
                let add_trailing = !no_newline
                    && match &value {
                        Value::List { vals, .. } => !vals.is_empty(),
                        Value::Record { val, .. } => !val.is_empty(),
                        _ => false,
                    };
                let mut str = local_into_string(engine_state, value, LINE_ENDING, serialize_types);
                if add_trailing {
                    str.push_str(LINE_ENDING);
                }
                Ok(
                    Value::string(str, head)
                        .into_pipeline_data_with_metadata(update_metadata(None)),
                )
            }
            PipelineData::ListStream(stream, meta) => {
                let span = stream.span();
                let from_io_error = IoError::factory(head, None);
                let stream = if no_newline {
                    let mut first = true;
                    let mut iter = stream.into_inner();
                    let engine_state_clone = engine_state.clone();
                    ByteStream::from_fn(
                        span,
                        engine_state.signals().clone(),
                        ByteStreamType::String,
                        move |buf| {
                            let Some(val) = iter.next() else {
                                return Ok(false);
                            };
                            if first {
                                first = false;
                            } else {
                                write!(buf, "{LINE_ENDING}").map_err(&from_io_error)?;
                            }
                            // TODO: write directly into `buf` instead of creating an intermediate
                            // string.
                            let str = local_into_string(
                                &engine_state_clone,
                                val,
                                LINE_ENDING,
                                serialize_types,
                            );
                            write!(buf, "{str}").map_err(&from_io_error)?;
                            Ok(true)
                        },
                    )
                } else {
                    let engine_state_clone = engine_state.clone();
                    ByteStream::from_iter(
                        stream.into_inner().map(move |val| {
                            let mut str = local_into_string(
                                &engine_state_clone,
                                val,
                                LINE_ENDING,
                                serialize_types,
                            );
                            str.push_str(LINE_ENDING);
                            str
                        }),
                        span,
                        engine_state.signals().clone(),
                        ByteStreamType::String,
                    )
                };

                Ok(PipelineData::byte_stream(stream, update_metadata(meta)))
            }
            PipelineData::ByteStream(stream, meta) => {
                Ok(PipelineData::byte_stream(stream, update_metadata(meta)))
            }
        }
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Outputs data as simple text with a trailing newline",
                example: "[1] | to text",
                result: Some(Value::test_string("1".to_string() + LINE_ENDING)),
            },
            Example {
                description: "Outputs data as simple text without a trailing newline",
                example: "[1] | to text --no-newline",
                result: Some(Value::test_string("1")),
            },
            Example {
                description: "Outputs external data as simple text",
                example: "git help -a | lines | find -r '^ ' | to text",
                result: None,
            },
            Example {
                description: "Outputs records as simple text",
                example: "ls | to text",
                result: None,
            },
        ]
    }
}

fn local_into_string(
    engine_state: &EngineState,
    value: Value,
    separator: &str,
    serialize_types: bool,
) -> String {
    let span = value.span();
    match value {
        Value::Bool { val, .. } => val.to_string(),
        Value::Int { val, .. } => val.to_string(),
        Value::Float { val, .. } => ObviousFloat(val).to_string(),
        Value::Filesize { val, .. } => val.to_string(),
        Value::Duration { val, .. } => format_duration(val),
        Value::Date { val, .. } => {
            format!(
                "{} ({})",
                {
                    if val.year() >= 0 {
                        val.to_rfc2822()
                    } else {
                        val.to_rfc3339()
                    }
                },
                HumanTime::from(val)
            )
        }
        Value::Range { val, .. } => val.to_string(),
        Value::String { val, .. } => val,
        Value::Glob { val, .. } => val,
        Value::List { vals: val, .. } => val
            .into_iter()
            .map(|x| local_into_string(engine_state, x, ", ", serialize_types))
            .collect::<Vec<_>>()
            .join(separator),
        Value::Record { val, .. } => val
            .into_owned()
            .into_iter()
            .map(|(x, y)| {
                format!(
                    "{}: {}",
                    x,
                    local_into_string(engine_state, y, ", ", serialize_types)
                )
            })
            .collect::<Vec<_>>()
            .join(separator),
        Value::Closure { val, .. } => {
            if serialize_types {
                let block = engine_state.get_block(val.block_id);
                if let Some(span) = block.span {
                    let contents_bytes = engine_state.get_span_contents(span);
                    let contents_string = String::from_utf8_lossy(contents_bytes);
                    contents_string.to_string()
                } else {
                    format!(
                        "unable to retrieve block contents for text block_id {}",
                        val.block_id.get()
                    )
                }
            } else {
                format!("closure_{}", val.block_id.get())
            }
        }
        Value::Nothing { .. } => String::new(),
        Value::Error { error, .. } => format!("{error:?}"),
        Value::Binary { val, .. } => format!("{val:?}"),
        Value::CellPath { val, .. } => val.to_string(),
        // If we fail to collapse the custom value, just print <{type_name}> - failure is not
        // that critical here
        Value::Custom { val, .. } => val
            .to_base_value(span)
            .map(|val| local_into_string(engine_state, val, separator, serialize_types))
            .unwrap_or_else(|_| format!("<{}>", val.type_name())),
    }
}

fn update_metadata(metadata: Option<PipelineMetadata>) -> Option<PipelineMetadata> {
    metadata
        .map(|md| md.with_content_type(Some(mime::TEXT_PLAIN.to_string())))
        .or_else(|| {
            Some(PipelineMetadata::default().with_content_type(Some(mime::TEXT_PLAIN.to_string())))
        })
}

#[cfg(test)]
mod test {
    use nu_cmd_lang::eval_pipeline_without_terminal_expression;

    use crate::{Get, Metadata};

    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(ToText {})
    }

    #[test]
    fn test_content_type_metadata() {
        let mut engine_state = Box::new(EngineState::new());
        let delta = {
            // Base functions that are needed for testing
            // Try to keep this working set small to keep tests running as fast as possible
            let mut working_set = StateWorkingSet::new(&engine_state);

            working_set.add_decl(Box::new(ToText {}));
            working_set.add_decl(Box::new(Metadata {}));
            working_set.add_decl(Box::new(Get {}));

            working_set.render()
        };

        engine_state
            .merge_delta(delta)
            .expect("Error merging delta");

        let cmd = "{a: 1 b: 2} | to text  | metadata | get content_type | $in";
        let result = eval_pipeline_without_terminal_expression(
            cmd,
            std::env::temp_dir().as_ref(),
            &mut engine_state,
        );
        assert_eq!(
            Value::test_string("text/plain"),
            result.expect("There should be a result")
        );
    }
}
