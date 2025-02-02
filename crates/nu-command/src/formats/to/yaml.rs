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
            description: "Outputs an YAML string representing the contents of this table",
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

pub fn value_to_yaml_value(
    engine_state: &EngineState,
    v: &Value,
    serialize_types: bool,
) -> Result<yaml_rust2::Yaml, ShellError> {
    Ok(match &v {
        Value::Bool { val, .. } => yaml_rust2::Yaml::Boolean(*val),
        Value::Int { val, .. } => yaml_rust2::Yaml::Integer(*val),
        Value::Filesize { val, .. } => yaml_rust2::Yaml::Integer(val.get()),
        Value::Duration { val, .. } => yaml_rust2::Yaml::String(val.to_string()),
        Value::Date { val, .. } => yaml_rust2::Yaml::String(val.to_string()),
        Value::Range { .. } => yaml_rust2::Yaml::Null,
        Value::Float { val, .. } => yaml_rust2::Yaml::Real(val.to_string()),
        Value::String { val, .. } | Value::Glob { val, .. } => {
            yaml_rust2::Yaml::String(val.clone())
        }
        Value::Record { val, .. } => {
            let mut m = yaml_rust2::yaml::Hash::new();
            for (k, v) in &**val {
                m.insert(
                    yaml_rust2::Yaml::String(k.clone()),
                    value_to_yaml_value(engine_state, v, serialize_types)?,
                );
            }
            yaml_rust2::Yaml::Hash(m)
        }
        Value::List { vals, .. } => {
            let mut out = vec![];

            for value in vals {
                out.push(value_to_yaml_value(engine_state, value, serialize_types)?);
            }

            yaml_rust2::Yaml::Array(out)
        }
        Value::Closure { val, .. } => {
            if serialize_types {
                let block = engine_state.get_block(val.block_id);
                if let Some(span) = block.span {
                    let contents_bytes = engine_state.get_span_contents(span);
                    let contents_string = String::from_utf8_lossy(contents_bytes);
                    yaml_rust2::Yaml::String(contents_string.to_string())
                } else {
                    yaml_rust2::Yaml::String(format!(
                        "unable to retrieve block contents for yaml block_id {}",
                        val.block_id.get()
                    ))
                }
            } else {
                yaml_rust2::Yaml::Null
            }
        }
        Value::Nothing { .. } => yaml_rust2::Yaml::Null,
        Value::Error { error, .. } => return Err(*error.clone()),
        Value::Binary { val, .. } => yaml_rust2::Yaml::Array(
            val.iter()
                .map(|x| yaml_rust2::Yaml::Integer(*x as i64))
                .collect(),
        ),
        Value::CellPath { val, .. } => yaml_rust2::Yaml::Array(
            val.members
                .iter()
                .map(|x| match &x {
                    PathMember::String { val, .. } => Ok(yaml_rust2::Yaml::String(val.clone())),
                    PathMember::Int { val, .. } => Ok(yaml_rust2::Yaml::Integer(*val as i64)),
                })
                .collect::<Result<Vec<yaml_rust2::Yaml>, ShellError>>()?,
        ),
        Value::Custom { .. } => yaml_rust2::Yaml::Null,
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
    match &yaml_value.into_string() {
        Some(serde_yml_string) => {
            Ok(Value::string(serde_yml_string, head)
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

        let cmd = "{a: 1 b: 2} | to yaml  | metadata | get content_type";
        let result = eval_pipeline_without_terminal_expression(
            cmd,
            std::env::temp_dir().as_ref(),
            &mut engine_state,
        );
        assert_eq!(
            Value::test_record(record!("content_type" => Value::test_string("application/yaml"))),
            result.expect("There should be a result")
        );
    }
}
