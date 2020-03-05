use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::{CoerceInto, ShellError};
use nu_protocol::{Primitive, ReturnSuccess, Signature, UnspannedPathMember, UntaggedValue, Value};

pub struct ToTOML;

impl WholeStreamCommand for ToTOML {
    fn name(&self) -> &str {
        "to-toml"
    }

    fn signature(&self) -> Signature {
        Signature::build("to-toml")
    }

    fn usage(&self) -> &str {
        "Convert table into .toml text"
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        to_toml(args, registry)
    }
}

pub fn value_to_toml_value(v: &Value) -> Result<toml::Value, ShellError> {
    Ok(match &v.value {
        UntaggedValue::Primitive(Primitive::Boolean(b)) => toml::Value::Boolean(*b),
        UntaggedValue::Primitive(Primitive::Bytes(b)) => toml::Value::Integer(*b as i64),
        UntaggedValue::Primitive(Primitive::Duration(d)) => toml::Value::Integer(*d as i64),
        UntaggedValue::Primitive(Primitive::Date(d)) => toml::Value::String(d.to_string()),
        UntaggedValue::Primitive(Primitive::EndOfStream) => {
            toml::Value::String("<End of Stream>".to_string())
        }
        UntaggedValue::Primitive(Primitive::BeginningOfStream) => {
            toml::Value::String("<Beginning of Stream>".to_string())
        }
        UntaggedValue::Primitive(Primitive::Decimal(f)) => {
            toml::Value::Float(f.tagged(&v.tag).coerce_into("converting to TOML float")?)
        }
        UntaggedValue::Primitive(Primitive::Int(i)) => {
            toml::Value::Integer(i.tagged(&v.tag).coerce_into("converting to TOML integer")?)
        }
        UntaggedValue::Primitive(Primitive::Nothing) => {
            toml::Value::String("<Nothing>".to_string())
        }
        UntaggedValue::Primitive(Primitive::Pattern(s)) => toml::Value::String(s.clone()),
        UntaggedValue::Primitive(Primitive::String(s)) => toml::Value::String(s.clone()),
        UntaggedValue::Primitive(Primitive::Line(s)) => toml::Value::String(s.clone()),
        UntaggedValue::Primitive(Primitive::Path(s)) => {
            toml::Value::String(s.display().to_string())
        }
        UntaggedValue::Primitive(Primitive::ColumnPath(path)) => toml::Value::Array(
            path.iter()
                .map(|x| match &x.unspanned {
                    UnspannedPathMember::String(string) => Ok(toml::Value::String(string.clone())),
                    UnspannedPathMember::Int(int) => Ok(toml::Value::Integer(
                        int.tagged(&v.tag)
                            .coerce_into("converting to TOML integer")?,
                    )),
                })
                .collect::<Result<Vec<toml::Value>, ShellError>>()?,
        ),

        UntaggedValue::Table(l) => toml::Value::Array(collect_values(l)?),
        UntaggedValue::Error(e) => return Err(e.clone()),
        UntaggedValue::Block(_) => toml::Value::String("<Block>".to_string()),
        UntaggedValue::Primitive(Primitive::Range(_)) => toml::Value::String("<Range>".to_string()),
        UntaggedValue::Primitive(Primitive::Binary(b)) => {
            toml::Value::Array(b.iter().map(|x| toml::Value::Integer(*x as i64)).collect())
        }
        UntaggedValue::Row(o) => {
            let mut m = toml::map::Map::new();
            for (k, v) in o.entries.iter() {
                m.insert(k.clone(), value_to_toml_value(v)?);
            }
            toml::Value::Table(m)
        }
    })
}

fn collect_values(input: &[Value]) -> Result<Vec<toml::Value>, ShellError> {
    let mut out = vec![];

    for value in input {
        out.push(value_to_toml_value(value)?);
    }

    Ok(out)
}

fn to_toml(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;
    let name_tag = args.name_tag();
    let name_span = name_tag.span;
    let stream = async_stream! {
        let input: Vec<Value> = args.input.values.collect().await;

        let to_process_input = if input.len() > 1 {
            let tag = input[0].tag.clone();
            vec![Value { value: UntaggedValue::Table(input), tag } ]
        } else if input.len() == 1 {
            input
        } else {
            vec![]
        };

        for value in to_process_input {
            let value_span = value.tag.span;
            match value_to_toml_value(&value) {
                Ok(toml_value) => {
                    match toml::to_string(&toml_value) {
                        Ok(x) => yield ReturnSuccess::value(
                            UntaggedValue::Primitive(Primitive::String(x)).into_value(&name_tag),
                        ),
                        _ => yield Err(ShellError::labeled_error_with_secondary(
                            "Expected a table with TOML-compatible structure.tag() from pipeline",
                            "requires TOML-compatible input",
                            name_span,
                            "originates from here".to_string(),
                            value_span,
                        )),
                    }
                }
                _ => yield Err(ShellError::labeled_error(
                    "Expected a table with TOML-compatible structure from pipeline",
                    "requires TOML-compatible input",
                    &name_tag))
            }
        }
    };

    Ok(stream.to_output_stream())
}
