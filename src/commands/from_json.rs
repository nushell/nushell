use crate::object::{Primitive, Value, Dictionary, DataDescriptor};
use crate::prelude::*;

fn convert_json_value_to_nu_value(v: &serde_json::Value) -> Value {
    match v {
        serde_json::Value::Null => Value::Primitive(Primitive::String("".to_string())),
        serde_json::Value::Bool(b) => Value::Primitive(Primitive::Boolean(*b)),
        serde_json::Value::Number(n) => Value::Primitive(Primitive::Int(n.as_i64().unwrap())),
        serde_json::Value::String(s) => Value::Primitive(Primitive::String(s.clone())),
        serde_json::Value::Array(a) => Value::List(a.iter().map(|x| convert_json_value_to_nu_value(x)).collect()),
        serde_json::Value::Object(o) => {
            let mut collected = Dictionary::default();
            for (k, v) in o.iter() {
                collected.add(DataDescriptor::from(k.clone()), convert_json_value_to_nu_value(v));
            }
            Value::Object(collected)
        }
    }
}

pub fn from_json_string_to_value(s: String) -> Value {
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    convert_json_value_to_nu_value(&v)
}

pub fn from_json(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let out = args.input;
    Ok(out
        .map(|a| match a {
            Value::Primitive(Primitive::String(s)) => {
                ReturnValue::Value(from_json_string_to_value(s))
            }
            _ => ReturnValue::Value(Value::Primitive(Primitive::String("".to_string()))),
        })
        .boxed())
}
