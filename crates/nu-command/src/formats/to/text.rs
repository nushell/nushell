use chrono_humanize::HumanTime;
use nu_engine::command_prelude::*;
use nu_protocol::{
    format_duration, format_filesize_from_conf, ByteStream, Config, PipelineMetadata,
};

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
            .category(Category::Formats)
    }

    fn usage(&self) -> &str {
        "Converts data into simple text."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;
        let input = input.try_expand_range()?;
        let config = stack.get_config(engine_state);

        match input {
            PipelineData::Empty => Ok(Value::string(String::new(), span)
                .into_pipeline_data_with_metadata(update_metadata(None))),
            PipelineData::Value(value, ..) => {
                let str = local_into_string(value, LINE_ENDING, &config);
                Ok(
                    Value::string(str, span)
                        .into_pipeline_data_with_metadata(update_metadata(None)),
                )
            }
            PipelineData::ListStream(stream, meta) => {
                let span = stream.span();
                let iter = stream.into_inner().map(move |value| {
                    let mut str = local_into_string(value, LINE_ENDING, &config);
                    str.push_str(LINE_ENDING);
                    str
                });
                Ok(PipelineData::ByteStream(
                    ByteStream::from_iter(
                        iter,
                        span,
                        engine_state.signals().clone(),
                        ByteStreamType::String,
                    ),
                    update_metadata(meta),
                ))
            }
            PipelineData::ByteStream(stream, meta) => {
                Ok(PipelineData::ByteStream(stream, update_metadata(meta)))
            }
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Outputs data as simple text",
                example: "1 | to text",
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

fn local_into_string(value: Value, separator: &str, config: &Config) -> String {
    let span = value.span();
    match value {
        Value::Bool { val, .. } => val.to_string(),
        Value::Int { val, .. } => val.to_string(),
        Value::Float { val, .. } => val.to_string(),
        Value::Filesize { val, .. } => format_filesize_from_conf(val, config),
        Value::Duration { val, .. } => format_duration(val),
        Value::Date { val, .. } => {
            format!("{} ({})", val.to_rfc2822(), HumanTime::from(val))
        }
        Value::Range { val, .. } => val.to_string(),
        Value::String { val, .. } => val,
        Value::Glob { val, .. } => val,
        Value::List { vals: val, .. } => val
            .into_iter()
            .map(|x| local_into_string(x, ", ", config))
            .collect::<Vec<_>>()
            .join(separator),
        Value::Record { val, .. } => val
            .into_owned()
            .into_iter()
            .map(|(x, y)| format!("{}: {}", x, local_into_string(y, ", ", config)))
            .collect::<Vec<_>>()
            .join(separator),
        Value::Closure { val, .. } => format!("<Closure {}>", val.block_id),
        Value::Nothing { .. } => String::new(),
        Value::Error { error, .. } => format!("{error:?}"),
        Value::Binary { val, .. } => format!("{val:?}"),
        Value::CellPath { val, .. } => val.to_string(),
        // If we fail to collapse the custom value, just print <{type_name}> - failure is not
        // that critical here
        Value::Custom { val, .. } => val
            .to_base_value(span)
            .map(|val| local_into_string(val, separator, config))
            .unwrap_or_else(|_| format!("<{}>", val.type_name())),
    }
}

fn update_metadata(metadata: Option<PipelineMetadata>) -> Option<PipelineMetadata> {
    metadata
        .map(|md| md.with_content_type(Some("text/plain".to_string())))
        .or_else(|| {
            Some(PipelineMetadata::default().with_content_type(Some("text/plain".to_string())))
        })
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(ToText {})
    }
}
