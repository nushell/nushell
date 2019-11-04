use crate::commands::WholeStreamCommand;
use crate::data::{Primitive, Value};
use crate::prelude::*;
use crate::RawPathMember;

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

pub fn value_to_toml_value(v: &Tagged<Value>) -> Result<toml::Value, ShellError> {
    Ok(match v.item() {
        Value::Primitive(Primitive::Boolean(b)) => toml::Value::Boolean(*b),
        Value::Primitive(Primitive::Bytes(b)) => toml::Value::Integer(*b as i64),
        Value::Primitive(Primitive::Duration(d)) => toml::Value::Integer(*d as i64),
        Value::Primitive(Primitive::Date(d)) => toml::Value::String(d.to_string()),
        Value::Primitive(Primitive::EndOfStream) => {
            toml::Value::String("<End of Stream>".to_string())
        }
        Value::Primitive(Primitive::BeginningOfStream) => {
            toml::Value::String("<Beginning of Stream>".to_string())
        }
        Value::Primitive(Primitive::Decimal(f)) => {
            toml::Value::Float(f.tagged(&v.tag).coerce_into("converting to TOML float")?)
        }
        Value::Primitive(Primitive::Int(i)) => {
            toml::Value::Integer(i.tagged(&v.tag).coerce_into("converting to TOML integer")?)
        }
        Value::Primitive(Primitive::Nothing) => toml::Value::String("<Nothing>".to_string()),
        Value::Primitive(Primitive::Pattern(s)) => toml::Value::String(s.clone()),
        Value::Primitive(Primitive::String(s)) => toml::Value::String(s.clone()),
        Value::Primitive(Primitive::Path(s)) => toml::Value::String(s.display().to_string()),
        Value::Primitive(Primitive::ColumnPath(path)) => toml::Value::Array(
            path.iter()
                .map(|x| match &x.item {
                    RawPathMember::String(string) => Ok(toml::Value::String(string.clone())),
                    RawPathMember::Int(int) => Ok(toml::Value::Integer(
                        int.tagged(&v.tag)
                            .coerce_into("converting to TOML integer")?,
                    )),
                })
                .collect::<Result<Vec<toml::Value>, ShellError>>()?,
        ),

        Value::Table(l) => toml::Value::Array(collect_values(l)?),
        Value::Error(e) => return Err(e.clone()),
        Value::Block(_) => toml::Value::String("<Block>".to_string()),
        Value::Primitive(Primitive::Binary(b)) => {
            toml::Value::Array(b.iter().map(|x| toml::Value::Integer(*x as i64)).collect())
        }
        Value::Row(o) => {
            let mut m = toml::map::Map::new();
            for (k, v) in o.entries.iter() {
                m.insert(k.clone(), value_to_toml_value(v)?);
            }
            toml::Value::Table(m)
        }
    })
}

fn collect_values(input: &Vec<Tagged<Value>>) -> Result<Vec<toml::Value>, ShellError> {
    let mut out = vec![];

    for value in input {
        out.push(value_to_toml_value(value)?);
    }

    Ok(out)
}

fn to_toml(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
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
            match value_to_toml_value(&value) {
                Ok(toml_value) => {
                    match toml::to_string(&toml_value) {
                        Ok(x) => yield ReturnSuccess::value(
                            Value::Primitive(Primitive::String(x)).tagged(&name_tag),
                        ),
                        _ => yield Err(ShellError::labeled_error_with_secondary(
                            "Expected a table with TOML-compatible structure.tag() from pipeline",
                            "requires TOML-compatible input",
                            &name_tag,
                            "originates from here".to_string(),
                            value.tag(),
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
