use crate::commands::WholeStreamCommand;
use crate::object::{Primitive, Value};
use crate::prelude::*;

pub struct ToJSON;

impl WholeStreamCommand for ToJSON {
    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        to_json(args, registry)
    }

    fn name(&self) -> &str {
        "to-json"
    }

    fn signature(&self) -> Signature {
        Signature::build("to-json")
    }
}

pub fn value_to_json_value(v: &Value) -> serde_json::Value {
    match v {
        Value::Primitive(Primitive::Boolean(b)) => serde_json::Value::Bool(*b),
        Value::Primitive(Primitive::Bytes(b)) => {
            serde_json::Value::Number(serde_json::Number::from(*b as u64))
        }
        Value::Primitive(Primitive::Date(d)) => serde_json::Value::String(d.to_string()),
        Value::Primitive(Primitive::EndOfStream) => serde_json::Value::Null,
        Value::Primitive(Primitive::BeginningOfStream) => serde_json::Value::Null,
        Value::Primitive(Primitive::Float(f)) => {
            serde_json::Value::Number(serde_json::Number::from_f64(f.into_inner()).unwrap())
        }
        Value::Primitive(Primitive::Int(i)) => {
            serde_json::Value::Number(serde_json::Number::from(*i))
        }
        Value::Primitive(Primitive::Nothing) => serde_json::Value::Null,
        Value::Primitive(Primitive::String(s)) => serde_json::Value::String(s.clone()),
        Value::Primitive(Primitive::Path(s)) => serde_json::Value::String(s.display().to_string()),

        Value::List(l) => {
            serde_json::Value::Array(l.iter().map(|x| value_to_json_value(x)).collect())
        }
        Value::Block(_) => serde_json::Value::Null,
        Value::Binary(b) => serde_json::Value::Array(
            b.iter()
                .map(|x| {
                    serde_json::Value::Number(serde_json::Number::from_f64(*x as f64).unwrap())
                })
                .collect(),
        ),
        Value::Object(o) => {
            let mut m = serde_json::Map::new();
            for (k, v) in o.entries.iter() {
                m.insert(k.clone(), value_to_json_value(v));
            }
            serde_json::Value::Object(m)
        }
    }
}

fn to_json(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;
    let name_span = args.name_span();
    let out = args.input;

    Ok(out
        .values
        .map(
            move |a| match serde_json::to_string(&value_to_json_value(&a)) {
                Ok(x) => ReturnSuccess::value(
                    Value::Primitive(Primitive::String(x)).simple_spanned(name_span),
                ),
                _ => Err(ShellError::labeled_error_with_secondary(
                    "Expected an object with JSON-compatible structure from pipeline",
                    "requires JSON-compatible input",
                    name_span,
                    format!("{} originates from here", a.item.type_name()),
                    a.span(),
                )),
            },
        )
        .to_output_stream())
}
