use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::{CoerceInto, ShellError};
use nu_protocol::{Primitive, ReturnSuccess, Signature, UnspannedPathMember, UntaggedValue, Value};

pub struct ToYAML;

#[async_trait]
impl WholeStreamCommand for ToYAML {
    fn name(&self) -> &str {
        "to yaml"
    }

    fn signature(&self) -> Signature {
        Signature::build("to yaml")
    }

    fn usage(&self) -> &str {
        "Convert table into .yaml/.yml text"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        to_yaml(args).await
    }
}

pub fn value_to_yaml_value(v: &Value) -> Result<serde_yaml::Value, ShellError> {
    Ok(match &v.value {
        UntaggedValue::Primitive(Primitive::Boolean(b)) => serde_yaml::Value::Bool(*b),
        UntaggedValue::Primitive(Primitive::Filesize(b)) => {
            serde_yaml::Value::Number(serde_yaml::Number::from(b.to_f64().ok_or_else(|| {
                ShellError::labeled_error(
                    "Could not convert to bytes",
                    "could not convert to bytes",
                    &v.tag,
                )
            })?))
        }
        UntaggedValue::Primitive(Primitive::Duration(i)) => {
            serde_yaml::Value::String(i.to_string())
        }
        UntaggedValue::Primitive(Primitive::Date(d)) => serde_yaml::Value::String(d.to_string()),
        UntaggedValue::Primitive(Primitive::EndOfStream) => serde_yaml::Value::Null,
        UntaggedValue::Primitive(Primitive::BeginningOfStream) => serde_yaml::Value::Null,
        UntaggedValue::Primitive(Primitive::Decimal(f)) => {
            serde_yaml::Value::Number(serde_yaml::Number::from(f.to_f64().ok_or_else(|| {
                ShellError::labeled_error(
                    "Could not convert to decimal",
                    "could not convert to decimal",
                    &v.tag,
                )
            })?))
        }
        UntaggedValue::Primitive(Primitive::Int(i)) => {
            serde_yaml::Value::Number(serde_yaml::Number::from(CoerceInto::<i64>::coerce_into(
                i.tagged(&v.tag),
                "converting to YAML number",
            )?))
        }
        UntaggedValue::Primitive(Primitive::Nothing) => serde_yaml::Value::Null,
        UntaggedValue::Primitive(Primitive::GlobPattern(s)) => serde_yaml::Value::String(s.clone()),
        UntaggedValue::Primitive(Primitive::String(s)) => serde_yaml::Value::String(s.clone()),
        UntaggedValue::Primitive(Primitive::ColumnPath(path)) => {
            let mut out = vec![];

            for member in path.iter() {
                match &member.unspanned {
                    UnspannedPathMember::String(string) => {
                        out.push(serde_yaml::Value::String(string.clone()))
                    }
                    UnspannedPathMember::Int(int) => out.push(serde_yaml::Value::Number(
                        serde_yaml::Number::from(CoerceInto::<i64>::coerce_into(
                            int.tagged(&member.span),
                            "converting to YAML number",
                        )?),
                    )),
                }
            }

            serde_yaml::Value::Sequence(out)
        }
        UntaggedValue::Primitive(Primitive::FilePath(s)) => {
            serde_yaml::Value::String(s.display().to_string())
        }

        UntaggedValue::Table(l) => {
            let mut out = vec![];

            for value in l {
                out.push(value_to_yaml_value(value)?);
            }

            serde_yaml::Value::Sequence(out)
        }
        UntaggedValue::Error(e) => return Err(e.clone()),
        UntaggedValue::Block(_) | UntaggedValue::Primitive(Primitive::Range(_)) => {
            serde_yaml::Value::Null
        }
        UntaggedValue::Primitive(Primitive::Binary(b)) => serde_yaml::Value::Sequence(
            b.iter()
                .map(|x| serde_yaml::Value::Number(serde_yaml::Number::from(*x)))
                .collect(),
        ),
        UntaggedValue::Row(o) => {
            let mut m = serde_yaml::Mapping::new();
            for (k, v) in o.entries.iter() {
                m.insert(
                    serde_yaml::Value::String(k.clone()),
                    value_to_yaml_value(v)?,
                );
            }
            serde_yaml::Value::Mapping(m)
        }
    })
}

async fn to_yaml(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once().await?;
    let name_tag = args.name_tag();
    let name_span = name_tag.span;

    let input: Vec<Value> = args.input.collect().await;

    let to_process_input = match input.len() {
        x if x > 1 => {
            let tag = input[0].tag.clone();
            vec![Value {
                value: UntaggedValue::Table(input),
                tag,
            }]
        }
        1 => input,
        _ => vec![],
    };

    Ok(
        futures::stream::iter(to_process_input.into_iter().map(move |value| {
            let value_span = value.tag.span;

            match value_to_yaml_value(&value) {
                Ok(yaml_value) => match serde_yaml::to_string(&yaml_value) {
                    Ok(x) => ReturnSuccess::value(
                        UntaggedValue::Primitive(Primitive::String(x)).into_value(&name_tag),
                    ),
                    _ => Err(ShellError::labeled_error_with_secondary(
                        "Expected a table with YAML-compatible structure from pipeline",
                        "requires YAML-compatible input",
                        name_span,
                        "originates from here".to_string(),
                        value_span,
                    )),
                },
                _ => Err(ShellError::labeled_error(
                    "Expected a table with YAML-compatible structure from pipeline",
                    "requires YAML-compatible input",
                    &name_tag,
                )),
            }
        }))
        .to_output_stream(),
    )
}

#[cfg(test)]
mod tests {
    use super::ShellError;
    use super::ToYAML;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(ToYAML {})
    }
}
