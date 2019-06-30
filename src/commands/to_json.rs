use crate::object::{Primitive, Value};
use crate::prelude::*;

pub fn value_to_json_value(v: &Value) -> serde_json::Value {
    match v {
        Value::Primitive(Primitive::Boolean(b)) => serde_json::Value::Bool(*b),
        Value::Primitive(Primitive::Bytes(b)) => {
            serde_json::Value::Number(serde_json::Number::from(*b as u64))
        }
        Value::Primitive(Primitive::Date(d)) => serde_json::Value::String(d.to_string()),
        Value::Primitive(Primitive::EndOfStream) => serde_json::Value::Null,
        Value::Primitive(Primitive::Float(f)) => {
            serde_json::Value::Number(serde_json::Number::from_f64(f.into_inner()).unwrap())
        }
        Value::Primitive(Primitive::Int(i)) => {
            serde_json::Value::Number(serde_json::Number::from(*i))
        }
        Value::Primitive(Primitive::Nothing) => serde_json::Value::Null,
        Value::Primitive(Primitive::String(s)) => serde_json::Value::String(s.clone()),

        Value::Filesystem => serde_json::Value::Null,
        Value::List(l) => {
            serde_json::Value::Array(l.iter().map(|x| value_to_json_value(x)).collect())
        }
        Value::Error(e) => serde_json::Value::String(e.to_string()),
        Value::Block(_) => serde_json::Value::Null,
        Value::Object(o) => {
            let mut m = serde_json::Map::new();
            for (k, v) in o.entries.iter() {
                m.insert(k.name.display().to_string(), value_to_json_value(v));
            }
            serde_json::Value::Object(m)
        }
    }
}

pub fn to_json(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let out = args.input;
    let span = args.name_span;
    Ok(out
        .map(
            move |a| match serde_json::to_string(&value_to_json_value(&a)) {
                Ok(x) => ReturnValue::Value(Value::Primitive(Primitive::String(x))),
                Err(_) => {
                    ReturnValue::Value(Value::Error(Box::new(ShellError::maybe_labeled_error(
                        "Can not convert to JSON string",
                        "can not convert piped data to JSON string",
                        span,
                    ))))
                }
            },
        )
        .boxed())
}
