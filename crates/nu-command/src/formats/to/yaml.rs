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
            .category(Category::Formats)
    }

    fn usage(&self) -> &str {
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
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let input = input.try_expand_range()?;
        to_yaml(input, head)
    }
}

pub fn value_to_yaml_value(v: &Value) -> Result<serde_yaml::Value, ShellError> {
    Ok(match &v {
        Value::Bool { val, .. } => serde_yaml::Value::Bool(*val),
        Value::Int { val, .. } => serde_yaml::Value::Number(serde_yaml::Number::from(*val)),
        Value::Filesize { val, .. } => serde_yaml::Value::Number(serde_yaml::Number::from(*val)),
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
                    value_to_yaml_value(v)?,
                );
            }
            serde_yaml::Value::Mapping(m)
        }
        Value::LazyRecord { val, .. } => {
            let collected = val.collect()?;
            value_to_yaml_value(&collected)?
        }
        Value::List { vals, .. } => {
            let mut out = vec![];

            for value in vals {
                out.push(value_to_yaml_value(value)?);
            }

            serde_yaml::Value::Sequence(out)
        }
        Value::Closure { .. } => serde_yaml::Value::Null,
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

fn to_yaml(input: PipelineData, head: Span) -> Result<PipelineData, ShellError> {
    let value = input.into_value(head);

    let yaml_value = value_to_yaml_value(&value)?;
    match serde_yaml::to_string(&yaml_value) {
        Ok(serde_yaml_string) => Ok(Value::string(serde_yaml_string, head).into_pipeline_data()),
        _ => Ok(Value::error(
            ShellError::CantConvert {
                to_type: "YAML".into(),
                from_type: value.get_type().to_string(),
                span: head,
                help: None,
            },
            head,
        )
        .into_pipeline_data()),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(ToYaml {})
    }
}
