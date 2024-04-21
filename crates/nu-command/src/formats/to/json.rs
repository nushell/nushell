use nu_engine::command_prelude::*;
use nu_protocol::ast::PathMember;

#[derive(Clone)]
pub struct ToJson;

impl Command for ToJson {
    fn name(&self) -> &str {
        "to json"
    }

    fn signature(&self) -> Signature {
        Signature::build("to json")
            .input_output_types(vec![(Type::Any, Type::String)])
            .switch("raw", "remove all of the whitespace", Some('r'))
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
            .category(Category::Formats)
    }

    fn usage(&self) -> &str {
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

        let span = call.head;
        // allow ranges to expand and turn into array
        let input = input.try_expand_range()?;
        let value = input.into_value(span);
        let json_value = value_to_json_value(&value)?;

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
                Ok(Value::string(serde_json_string, span).into_pipeline_data())
            }
            _ => Ok(Value::error(
                ShellError::CantConvert {
                    to_type: "JSON".into(),
                    from_type: value.get_type().to_string(),
                    span,
                    help: None,
                },
                span,
            )
            .into_pipeline_data()),
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description:
                    "Outputs a JSON string, with default indentation, representing the contents of this table",
                example: "[a b c] | to json",
                result: Some(Value::test_string("[\n  \"a\",\n  \"b\",\n  \"c\"\n]")),
            },
            Example {
                description:
                    "Outputs a JSON string, with 4-space indentation, representing the contents of this table",
                example: "[Joe Bob Sam] | to json --indent 4",
                result: Some(Value::test_string("[\n    \"Joe\",\n    \"Bob\",\n    \"Sam\"\n]")),
            },
            Example {
                description:
                    "Outputs an unformatted JSON string representing the contents of this table",
                example: "[1 2 3] | to json -r",
                result: Some(Value::test_string("[1,2,3]")),
            },
        ]
    }
}

pub fn value_to_json_value(v: &Value) -> Result<nu_json::Value, ShellError> {
    let span = v.span();
    Ok(match v {
        Value::Bool { val, .. } => nu_json::Value::Bool(*val),
        Value::Filesize { val, .. } => nu_json::Value::I64(*val),
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

        Value::List { vals, .. } => nu_json::Value::Array(json_list(vals)?),
        Value::Error { error, .. } => return Err(*error.clone()),
        Value::Closure { .. } | Value::Range { .. } => nu_json::Value::Null,
        Value::Binary { val, .. } => {
            nu_json::Value::Array(val.iter().map(|x| nu_json::Value::U64(*x as u64)).collect())
        }
        Value::Record { val, .. } => {
            let mut m = nu_json::Map::new();
            for (k, v) in &**val {
                m.insert(k.clone(), value_to_json_value(v)?);
            }
            nu_json::Value::Object(m)
        }
        Value::LazyRecord { val, .. } => {
            let collected = val.collect()?;
            value_to_json_value(&collected)?
        }
        Value::Custom { val, .. } => {
            let collected = val.to_base_value(span)?;
            value_to_json_value(&collected)?
        }
    })
}

fn json_list(input: &[Value]) -> Result<Vec<nu_json::Value>, ShellError> {
    let mut out = vec![];

    for value in input {
        out.push(value_to_json_value(value)?);
    }

    Ok(out)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(ToJson {})
    }
}
