use crate::commands::WholeStreamCommand;
use crate::data::{Primitive, Value};
use crate::prelude::*;
use crate::RawPathMember;

pub struct ToJSON;

impl WholeStreamCommand for ToJSON {
    fn name(&self) -> &str {
        "to-json"
    }

    fn signature(&self) -> Signature {
        Signature::build("to-json")
    }

    fn usage(&self) -> &str {
        "Convert table into .json text"
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        to_json(args, registry)
    }
}

pub fn value_to_json_value(v: &Tagged<Value>) -> Result<serde_json::Value, ShellError> {
    Ok(match v.item() {
        Value::Primitive(Primitive::Boolean(b)) => serde_json::Value::Bool(*b),
        Value::Primitive(Primitive::Bytes(b)) => serde_json::Value::Number(
            serde_json::Number::from(b.to_u64().expect("What about really big numbers")),
        ),
        Value::Primitive(Primitive::Duration(secs)) => {
            serde_json::Value::Number(serde_json::Number::from(*secs))
        }
        Value::Primitive(Primitive::Date(d)) => serde_json::Value::String(d.to_string()),
        Value::Primitive(Primitive::EndOfStream) => serde_json::Value::Null,
        Value::Primitive(Primitive::BeginningOfStream) => serde_json::Value::Null,
        Value::Primitive(Primitive::Decimal(f)) => serde_json::Value::Number(
            serde_json::Number::from_f64(
                f.to_f64().expect("TODO: What about really big decimals?"),
            )
            .unwrap(),
        ),
        Value::Primitive(Primitive::Int(i)) => serde_json::Value::Number(serde_json::Number::from(
            CoerceInto::<i64>::coerce_into(i.tagged(&v.tag), "converting to JSON number")?,
        )),
        Value::Primitive(Primitive::Nothing) => serde_json::Value::Null,
        Value::Primitive(Primitive::Pattern(s)) => serde_json::Value::String(s.clone()),
        Value::Primitive(Primitive::String(s)) => serde_json::Value::String(s.clone()),
        Value::Primitive(Primitive::ColumnPath(path)) => serde_json::Value::Array(
            path.iter()
                .map(|x| match &x.item {
                    RawPathMember::String(string) => Ok(serde_json::Value::String(string.clone())),
                    RawPathMember::Int(int) => Ok(serde_json::Value::Number(
                        serde_json::Number::from(CoerceInto::<i64>::coerce_into(
                            int.tagged(&v.tag),
                            "converting to JSON number",
                        )?),
                    )),
                })
                .collect::<Result<Vec<serde_json::Value>, ShellError>>()?,
        ),
        Value::Primitive(Primitive::Path(s)) => serde_json::Value::String(s.display().to_string()),

        Value::Table(l) => serde_json::Value::Array(json_list(l)?),
        Value::Error(e) => return Err(e.clone()),
        Value::Block(_) => serde_json::Value::Null,
        Value::Primitive(Primitive::Binary(b)) => serde_json::Value::Array(
            b.iter()
                .map(|x| {
                    serde_json::Value::Number(serde_json::Number::from_f64(*x as f64).unwrap())
                })
                .collect(),
        ),
        Value::Row(o) => {
            let mut m = serde_json::Map::new();
            for (k, v) in o.entries.iter() {
                m.insert(k.clone(), value_to_json_value(v)?);
            }
            serde_json::Value::Object(m)
        }
    })
}

fn json_list(input: &Vec<Tagged<Value>>) -> Result<Vec<serde_json::Value>, ShellError> {
    let mut out = vec![];

    for value in input {
        out.push(value_to_json_value(value)?);
    }

    Ok(out)
}

fn to_json(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
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
            match value_to_json_value(&value) {
                Ok(json_value) => {
                    match serde_json::to_string(&json_value) {
                        Ok(x) => yield ReturnSuccess::value(
                            Value::Primitive(Primitive::String(x)).tagged(&name_tag),
                        ),
                        _ => yield Err(ShellError::labeled_error_with_secondary(
                            "Expected a table with JSON-compatible structure.tag() from pipeline",
                            "requires JSON-compatible input",
                            &name_tag,
                            "originates from here".to_string(),
                            value.tag(),
                        )),
                    }
                }
                _ => yield Err(ShellError::labeled_error(
                    "Expected a table with JSON-compatible structure from pipeline",
                    "requires JSON-compatible input",
                    &name_tag))
            }
        }
    };

    Ok(stream.to_output_stream())
}
