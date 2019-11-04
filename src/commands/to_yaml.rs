use crate::commands::WholeStreamCommand;
use crate::data::{Primitive, Value};
use crate::prelude::*;
use crate::RawPathMember;

pub struct ToYAML;

impl WholeStreamCommand for ToYAML {
    fn name(&self) -> &str {
        "to-yaml"
    }

    fn signature(&self) -> Signature {
        Signature::build("to-yaml")
    }

    fn usage(&self) -> &str {
        "Convert table into .yaml/.yml text"
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        to_yaml(args, registry)
    }
}

pub fn value_to_yaml_value(v: &Tagged<Value>) -> Result<serde_yaml::Value, ShellError> {
    Ok(match v.item() {
        Value::Primitive(Primitive::Boolean(b)) => serde_yaml::Value::Bool(*b),
        Value::Primitive(Primitive::Bytes(b)) => {
            serde_yaml::Value::Number(serde_yaml::Number::from(b.to_f64().unwrap()))
        }
        Value::Primitive(Primitive::Duration(secs)) => {
            serde_yaml::Value::Number(serde_yaml::Number::from(secs.to_f64().unwrap()))
        }
        Value::Primitive(Primitive::Date(d)) => serde_yaml::Value::String(d.to_string()),
        Value::Primitive(Primitive::EndOfStream) => serde_yaml::Value::Null,
        Value::Primitive(Primitive::BeginningOfStream) => serde_yaml::Value::Null,
        Value::Primitive(Primitive::Decimal(f)) => {
            serde_yaml::Value::Number(serde_yaml::Number::from(f.to_f64().unwrap()))
        }
        Value::Primitive(Primitive::Int(i)) => serde_yaml::Value::Number(serde_yaml::Number::from(
            CoerceInto::<i64>::coerce_into(i.tagged(&v.tag), "converting to YAML number")?,
        )),
        Value::Primitive(Primitive::Nothing) => serde_yaml::Value::Null,
        Value::Primitive(Primitive::Pattern(s)) => serde_yaml::Value::String(s.clone()),
        Value::Primitive(Primitive::String(s)) => serde_yaml::Value::String(s.clone()),
        Value::Primitive(Primitive::ColumnPath(path)) => {
            let mut out = vec![];

            for member in path.iter() {
                match &member.item {
                    RawPathMember::String(string) => {
                        out.push(serde_yaml::Value::String(string.clone()))
                    }
                    RawPathMember::Int(int) => out.push(serde_yaml::Value::Number(
                        serde_yaml::Number::from(CoerceInto::<i64>::coerce_into(
                            int.tagged(&member.span),
                            "converting to YAML number",
                        )?),
                    )),
                }
            }

            serde_yaml::Value::Sequence(out)
        }
        Value::Primitive(Primitive::Path(s)) => serde_yaml::Value::String(s.display().to_string()),

        Value::Table(l) => {
            let mut out = vec![];

            for value in l {
                out.push(value_to_yaml_value(value)?);
            }

            serde_yaml::Value::Sequence(out)
        }
        Value::Error(e) => return Err(e.clone()),
        Value::Block(_) => serde_yaml::Value::Null,
        Value::Primitive(Primitive::Binary(b)) => serde_yaml::Value::Sequence(
            b.iter()
                .map(|x| serde_yaml::Value::Number(serde_yaml::Number::from(*x)))
                .collect(),
        ),
        Value::Row(o) => {
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

fn to_yaml(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;
    let name_tag = args.name_tag();
    let stream = async_stream! {
        let input: Vec<Tagged<Value>> = args.input.values.collect().await;

        let to_process_input = if input.len() > 1 {
            let tag = input[0].tag.clone();
            vec![Tagged { item: Value::Table(input), tag } ]
        } else if input.len() == 1 {
            input
        } else {
            vec![]
        };

        for value in to_process_input {
            match value_to_yaml_value(&value) {
                Ok(yaml_value) => {
                    match serde_yaml::to_string(&yaml_value) {
                        Ok(x) => yield ReturnSuccess::value(
                            Value::Primitive(Primitive::String(x)).tagged(&name_tag),
                        ),
                        _ => yield Err(ShellError::labeled_error_with_secondary(
                            "Expected a table with YAML-compatible structure.tag() from pipeline",
                            "requires YAML-compatible input",
                            &name_tag,
                            "originates from here".to_string(),
                            value.tag(),
                        )),
                    }
                }
                _ => yield Err(ShellError::labeled_error(
                    "Expected a table with YAML-compatible structure from pipeline",
                    "requires YAML-compatible input",
                    &name_tag))
            }
        }
    };

    Ok(stream.to_output_stream())
}
