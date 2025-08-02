use nu_engine::command_prelude::*;
use nu_protocol::{PipelineMetadata, ast::PathMember};

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
                "remove all of the whitespace and trailing line ending",
                Some('r'),
            )
            .named(
                "indent",
                SyntaxShape::Number,
                "specify indentation width",
                Some('i'),
            )
            .named(
                "tabs",
                SyntaxShape::Number,
                "specify indentation tab quantity",
                Some('t'),
            )
            .switch(
                "serialize",
                "serialize nushell types that cannot be deserialized",
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
        let json_value = value_to_json_value(engine_state, &value, span, serialize_types)?;

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
                };
                Ok(PipelineData::value(res, Some(metadata)))
            }
            _ => Err(ShellError::CantConvert {
                to_type: "JSON".into(),
                from_type: value.get_type().to_string(),
                span,
                help: None,
            }),
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Outputs a JSON string, with default indentation, representing the contents of this table",
                example: "[a b c] | to json",
                result: Some(Value::test_string("[\n  \"a\",\n  \"b\",\n  \"c\"\n]")),
            },
            Example {
                description: "Outputs a JSON string, with 4-space indentation, representing the contents of this table",
                example: "[Joe Bob Sam] | to json --indent 4",
                result: Some(Value::test_string(
                    "[\n    \"Joe\",\n    \"Bob\",\n    \"Sam\"\n]",
                )),
            },
            Example {
                description: "Outputs an unformatted JSON string representing the contents of this table",
                example: "[1 2 3] | to json -r",
                result: Some(Value::test_string("[1,2,3]")),
            },
        ]
    }
}

pub fn value_to_json_value(
    engine_state: &EngineState,
    v: &Value,
    call_span: Span,
    serialize_types: bool,
) -> Result<nu_json::Value, ShellError> {
    let span = v.span();
    Ok(match v {
        Value::Bool { val, .. } => nu_json::Value::Bool(*val),
        Value::Filesize { val, .. } => nu_json::Value::I64(val.get()),
        Value::Duration { val, .. } => nu_json::Value::I64(*val),
        Value::Date { val, .. } => nu_json::Value::String(val.to_string()),
        Value::Float { val, .. } => nu_json::Value::F64(*val),
        Value::Int { val, .. } => nu_json::Value::I64(*val),
        Value::Nothing { .. } => nu_json::Value::Null,
        Value::String { val, .. } => nu_json::Value::String(val.to_string()),
        Value::Glob { val, .. } => nu_json::Value::String(val.to_string()),
        Value::CellPath { val, .. } => nu_json::Value::Array(
            val.members
                .iter()
                .map(|x| match &x {
                    PathMember::String { val, .. } => Ok(nu_json::Value::String(val.clone())),
                    PathMember::Int { val, .. } => Ok(nu_json::Value::U64(*val as u64)),
                })
                .collect::<Result<Vec<nu_json::Value>, ShellError>>()?,
        ),

        Value::List { vals, .. } => {
            nu_json::Value::Array(json_list(engine_state, vals, call_span, serialize_types)?)
        }
        Value::Error { error, .. } => return Err(*error.clone()),
        Value::Closure { val, .. } => {
            if serialize_types {
                let closure_string = val.coerce_into_string(engine_state, span)?;
                nu_json::Value::String(closure_string.to_string())
            } else {
                return Err(ShellError::UnsupportedInput {
                    msg: "closures are currently not deserializable (use --serialize to serialize as a string)".into(),
                    input: "value originates from here".into(),
                    msg_span: call_span,
                    input_span: span,
                });
            }
        }
        Value::Range { .. } => nu_json::Value::Null,
        Value::Binary { val, .. } => {
            nu_json::Value::Array(val.iter().map(|x| nu_json::Value::U64(*x as u64)).collect())
        }
        Value::Record { val, .. } => {
            let mut m = nu_json::Map::new();
            for (k, v) in &**val {
                m.insert(
                    k.clone(),
                    value_to_json_value(engine_state, v, call_span, serialize_types)?,
                );
            }
            nu_json::Value::Object(m)
        }
        Value::Custom { val, .. } => {
            let collected = val.to_base_value(span)?;
            value_to_json_value(engine_state, &collected, call_span, serialize_types)?
        }
    })
}

fn json_list(
    engine_state: &EngineState,
    input: &[Value],
    call_span: Span,
    serialize_types: bool,
) -> Result<Vec<nu_json::Value>, ShellError> {
    let mut out = vec![];

    for value in input {
        out.push(value_to_json_value(
            engine_state,
            value,
            call_span,
            serialize_types,
        )?);
    }

    Ok(out)
}

#[cfg(test)]
mod test {
    use nu_cmd_lang::eval_pipeline_without_terminal_expression;

    use crate::{Get, Metadata};

    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(ToJson {})
    }

    #[test]
    fn test_content_type_metadata() {
        let mut engine_state = Box::new(EngineState::new());
        let delta = {
            // Base functions that are needed for testing
            // Try to keep this working set small to keep tests running as fast as possible
            let mut working_set = StateWorkingSet::new(&engine_state);

            working_set.add_decl(Box::new(ToJson {}));
            working_set.add_decl(Box::new(Metadata {}));
            working_set.add_decl(Box::new(Get {}));

            working_set.render()
        };

        engine_state
            .merge_delta(delta)
            .expect("Error merging delta");

        let cmd = "{a: 1 b: 2} | to json  | metadata | get content_type | $in";
        let result = eval_pipeline_without_terminal_expression(
            cmd,
            std::env::temp_dir().as_ref(),
            &mut engine_state,
        );
        assert_eq!(
            Value::test_string("application/json"),
            result.expect("There should be a result")
        );
    }
}
