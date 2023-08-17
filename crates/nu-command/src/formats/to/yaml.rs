use nu_protocol::ast::{Call, PathMember};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, SpannedValue,
    Type,
};

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
            result: Some(SpannedValue::test_string("- foo: '1'\n  bar: '2'\n")),
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

pub fn value_to_yaml_value(v: &SpannedValue) -> Result<serde_yaml::Value, ShellError> {
    Ok(match &v {
        SpannedValue::Bool { val, .. } => serde_yaml::Value::Bool(*val),
        SpannedValue::Int { val, .. } => serde_yaml::Value::Number(serde_yaml::Number::from(*val)),
        SpannedValue::Filesize { val, .. } => {
            serde_yaml::Value::Number(serde_yaml::Number::from(*val))
        }
        SpannedValue::Duration { val, .. } => serde_yaml::Value::String(val.to_string()),
        SpannedValue::Date { val, .. } => serde_yaml::Value::String(val.to_string()),
        SpannedValue::Range { .. } => serde_yaml::Value::Null,
        SpannedValue::Float { val, .. } => {
            serde_yaml::Value::Number(serde_yaml::Number::from(*val))
        }
        SpannedValue::String { val, .. } => serde_yaml::Value::String(val.clone()),
        SpannedValue::Record { cols, vals, .. } => {
            let mut m = serde_yaml::Mapping::new();
            for (k, v) in cols.iter().zip(vals.iter()) {
                m.insert(
                    serde_yaml::Value::String(k.clone()),
                    value_to_yaml_value(v)?,
                );
            }
            serde_yaml::Value::Mapping(m)
        }
        SpannedValue::LazyRecord { val, .. } => {
            let collected = val.collect()?;
            value_to_yaml_value(&collected)?
        }
        SpannedValue::List { vals, .. } => {
            let mut out = vec![];

            for value in vals {
                out.push(value_to_yaml_value(value)?);
            }

            serde_yaml::Value::Sequence(out)
        }
        SpannedValue::Block { .. } => serde_yaml::Value::Null,
        SpannedValue::Closure { .. } => serde_yaml::Value::Null,
        SpannedValue::Nothing { .. } => serde_yaml::Value::Null,
        SpannedValue::Error { error, .. } => return Err(*error.clone()),
        SpannedValue::Binary { val, .. } => serde_yaml::Value::Sequence(
            val.iter()
                .map(|x| serde_yaml::Value::Number(serde_yaml::Number::from(*x)))
                .collect(),
        ),
        SpannedValue::CellPath { val, .. } => serde_yaml::Value::Sequence(
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
        SpannedValue::CustomValue { .. } => serde_yaml::Value::Null,
        SpannedValue::MatchPattern { .. } => serde_yaml::Value::Null,
    })
}

fn to_yaml(input: PipelineData, head: Span) -> Result<PipelineData, ShellError> {
    let value = input.into_value(head);

    let yaml_value = value_to_yaml_value(&value)?;
    match serde_yaml::to_string(&yaml_value) {
        Ok(serde_yaml_string) => Ok(SpannedValue::String {
            val: serde_yaml_string,
            span: head,
        }
        .into_pipeline_data()),
        _ => Ok(SpannedValue::Error {
            error: Box::new(ShellError::CantConvert {
                to_type: "YAML".into(),
                from_type: value.get_type().to_string(),
                span: head,
                help: None,
            }),
            span: head,
        }
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
