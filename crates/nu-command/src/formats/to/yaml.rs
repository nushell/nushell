use nu_engine::command_prelude::*;
use nu_protocol::ast::PathMember;

#[derive(Clone)]
pub struct ToYaml;

impl Command for ToYaml {
    fn name(&self) -> &str {
        "to yaml"
    }

    fn signature(&self) -> Signature {
        Signature::build("to yaml")
            .input_output_types(vec![(Type::Any, Type::String)])
            .switch(
                "serialize",
                "serialize nushell types that cannot be deserialized",
                Some('s'),
            )
            .category(Category::Formats)
    }

    fn description(&self) -> &str {
        "Convert table into .yaml/.yml text."
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Outputs a YAML string representing the contents of this table",
            example: r#"[[foo bar]; ["1" "2"]] | to yaml"#,
            result: Some(Value::test_string("- foo: '1'\n  bar: '2'\n")),
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let serialize_types = call.has_flag(engine_state, stack, "serialize")?;
        let input = input.try_expand_range()?;

        to_yaml(engine_state, input, head, serialize_types)
    }
}

#[derive(Clone)]
pub struct ToYml;

impl Command for ToYml {
    fn name(&self) -> &str {
        "to yml"
    }

    fn signature(&self) -> Signature {
        Signature::build("to yml")
            .input_output_types(vec![(Type::Any, Type::String)])
            .switch(
                "serialize",
                "serialize nushell types that cannot be deserialized",
                Some('s'),
            )
            .category(Category::Formats)
    }

    fn description(&self) -> &str {
        "Convert table into .yaml/.yml text."
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Outputs a YAML string representing the contents of this table",
            example: r#"[[foo bar]; ["1" "2"]] | to yml"#,
            result: Some(Value::test_string("- foo: '1'\n  bar: '2'\n")),
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let serialize_types = call.has_flag(engine_state, stack, "serialize")?;
        let input = input.try_expand_range()?;

        to_yaml(engine_state, input, head, serialize_types)
    }
}

pub fn value_to_yaml_value(
    engine_state: &EngineState,
    v: &Value,
    serialize_types: bool,
) -> Result<serde_yaml::Value, ShellError> {
    Ok(match &v {
        Value::Bool { val, .. } => serde_yaml::Value::Bool(*val),
        Value::Int { val, .. } => serde_yaml::Value::Number(serde_yaml::Number::from(*val)),
        Value::Filesize { val, .. } => {
            serde_yaml::Value::Number(serde_yaml::Number::from(val.get()))
        }
        Value::Duration { val, .. } => serde_yaml::Value::String(val.to_string()),
        Value::Date { val, .. } => serde_yaml::Value::String(val.to_string()),
        Value::Range { .. } => serde_yaml::Value::Null,
        Value::Float { val, .. } => serde_yaml::Value::Number(serde_yaml::Number::from(*val)),
        Value::String { val, .. } | Value::Glob { val, .. } => {
            serde_yaml::Value::String(val.clone())
        }
        Value::Record { val, .. } => {
            let mut m = serde_yaml::Mapping::new();
            for (k, v) in &**val {
                m.insert(
                    serde_yaml::Value::String(k.clone()),
                    value_to_yaml_value(engine_state, v, serialize_types)?,
                );
            }
            serde_yaml::Value::Mapping(m)
        }
        Value::List { vals, .. } => {
            let mut out = vec![];

            for value in vals {
                out.push(value_to_yaml_value(engine_state, value, serialize_types)?);
            }

            serde_yaml::Value::Sequence(out)
        }
        Value::Closure { val, .. } => {
            if serialize_types {
                let block = engine_state.get_block(val.block_id);
                if let Some(span) = block.span {
                    let contents_bytes = engine_state.get_span_contents(span);
                    let contents_string = String::from_utf8_lossy(contents_bytes);
                    serde_yaml::Value::String(contents_string.to_string())
                } else {
                    serde_yaml::Value::String(format!(
                        "unable to retrieve block contents for yaml block_id {}",
                        val.block_id.get()
                    ))
                }
            } else {
                serde_yaml::Value::Null
            }
        }
        Value::Nothing { .. } => serde_yaml::Value::Null,
        Value::Error { error, .. } => return Err(*error.clone()),
        Value::Binary { val, .. } => serde_yaml::Value::Sequence(
            val.iter()
                .map(|x| serde_yaml::Value::Number(serde_yaml::Number::from(*x)))
                .collect(),
        ),
        Value::CellPath { val, .. } => serde_yaml::Value::Sequence(
            val.members
                .iter()
                .map(|x| match &x {
                    PathMember::String { val, .. } => Ok(serde_yaml::Value::String(val.clone())),
                    PathMember::Int { val, .. } => {
                        Ok(serde_yaml::Value::Number(serde_yaml::Number::from(*val)))
                    }
                })
                .collect::<Result<Vec<serde_yaml::Value>, ShellError>>()?,
        ),
        Value::Custom { .. } => serde_yaml::Value::Null,
    })
}

fn to_yaml(
    engine_state: &EngineState,
    input: PipelineData,
    head: Span,
    serialize_types: bool,
) -> Result<PipelineData, ShellError> {
    let metadata = input
        .metadata()
        .unwrap_or_default()
        // Per RFC-9512, application/yaml should be used
        .with_content_type(Some("application/yaml".into()));
    let value = input.into_value(head)?;

    let yaml_value = value_to_yaml_value(engine_state, &value, serialize_types)?;
    match serde_yaml::to_string(&yaml_value) {
        Ok(serde_yaml_string) => {
            Ok(Value::string(serde_yaml_string, head)
                .into_pipeline_data_with_metadata(Some(metadata)))
        }
        _ => Ok(Value::error(
            ShellError::CantConvert {
                to_type: "YAML".into(),
                from_type: value.get_type().to_string(),
                span: head,
                help: None,
            },
            head,
        )
        .into_pipeline_data_with_metadata(Some(metadata))),
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{Get, Metadata};
    use nu_cmd_lang::eval_pipeline_without_terminal_expression;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(ToYaml {})
    }

    #[test]
    fn test_content_type_metadata() {
        let mut engine_state = Box::new(EngineState::new());
        let delta = {
            // Base functions that are needed for testing
            // Try to keep this working set small to keep tests running as fast as possible
            let mut working_set = StateWorkingSet::new(&engine_state);

            working_set.add_decl(Box::new(ToYaml {}));
            working_set.add_decl(Box::new(Metadata {}));
            working_set.add_decl(Box::new(Get {}));

            working_set.render()
        };

        engine_state
            .merge_delta(delta)
            .expect("Error merging delta");

        let cmd = "{a: 1 b: 2} | to yaml  | metadata | get content_type | $in";
        let result = eval_pipeline_without_terminal_expression(
            cmd,
            std::env::temp_dir().as_ref(),
            &mut engine_state,
        );
        assert_eq!(
            Value::test_string("application/yaml"),
            result.expect("There should be a result")
        );
    }
}
