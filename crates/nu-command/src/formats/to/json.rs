use nu_engine::command_prelude::*;
use nu_protocol::{FromValue, PipelineMetadata};

#[derive(Clone)]
pub struct ToJson;

impl Command for ToJson {
    fn name(&self) -> &str {
        "to json"
    }

    fn signature(&self) -> Signature {
        Signature::build("to json")
            .input_output_types(vec![(Type::Any, Type::String)])
            .switch(
                "raw",
                "Remove all of the whitespace and trailing line ending.",
                Some('r'),
            )
            .named(
                "indent",
                SyntaxShape::Number,
                "Specify indentation width.",
                Some('i'),
            )
            .named(
                "tabs",
                SyntaxShape::Number,
                "Specify indentation tab quantity.",
                Some('t'),
            )
            .switch(
                "serialize",
                "Serialize nushell types that cannot be deserialized.",
                Some('s'),
            )
            .category(Category::Formats)
    }

    fn description(&self) -> &str {
        "Converts table data into JSON text."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let raw = call.has_flag(engine_state, stack, "raw")?;
        let use_tabs = call.get_flag(engine_state, stack, "tabs")?;
        let indent = call.get_flag(engine_state, stack, "indent")?;
        let serialize_types = call.has_flag(engine_state, stack, "serialize")?;

        let span = call.head;
        // allow ranges to expand and turn into array
        let input = input.try_expand_range()?;
        let value = input.into_value(span)?;
        let ty = value.get_type();
        let json_value = value_to_json_value(engine_state, value, span, serialize_types)?;

        let json_result = if raw {
            nu_json::to_string_raw(&json_value)
        } else if let Some(tab_count) = use_tabs {
            nu_json::to_string_with_tab_indentation(&json_value, tab_count)
        } else if let Some(indent) = indent {
            nu_json::to_string_with_indent(&json_value, indent)
        } else {
            nu_json::to_string(&json_value)
        };

        match json_result {
            Ok(serde_json_string) => {
                let res = Value::string(serde_json_string, span);
                let metadata = PipelineMetadata {
                    data_source: nu_protocol::DataSource::None,
                    content_type: Some(mime::APPLICATION_JSON.to_string()),
                    ..Default::default()
                };
                Ok(PipelineData::value(res, Some(metadata)))
            }
            _ => Err(ShellError::CantConvert {
                to_type: "JSON".into(),
                from_type: ty.to_string(),
                span,
                help: None,
            }),
        }
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Outputs a JSON string, with default indentation, representing the contents of this table.",
                example: "[a b c] | to json",
                result: Some(Value::test_string("[\n  \"a\",\n  \"b\",\n  \"c\"\n]")),
            },
            Example {
                description: "Outputs a JSON string, with 4-space indentation, representing the contents of this table.",
                example: "[Joe Bob Sam] | to json --indent 4",
                result: Some(Value::test_string(
                    "[\n    \"Joe\",\n    \"Bob\",\n    \"Sam\"\n]",
                )),
            },
            Example {
                description: "Outputs an unformatted JSON string representing the contents of this table.",
                example: "[1 2 3] | to json -r",
                result: Some(Value::test_string("[1,2,3]")),
            },
        ]
    }
}

pub fn value_to_json_value(
    engine_state: &EngineState,
    v: Value,
    call_span: Span,
    serialize_types: bool,
) -> Result<nu_json::Value, ShellError> {
    let value_span = v.span();
    match serialize_types {
        false => nu_json::Value::from_value(v),
        true => nu_json::Value::from_value_serialized(v, engine_state)
    }.map_err(|err| match err {
        ShellError::CantConvert { from_type, .. } if from_type == "closure" => ShellError::UnsupportedInput {
            msg: "closures are currently not deserializable (use --serialize to serialize as a string)".into(),
            input: "value originates from here".into(),
            msg_span: call_span,
            input_span: value_span,
        },
        err => err
    })
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(ToJson)
    }
}
