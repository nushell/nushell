use crate::object::base::OF64;
use crate::object::{DataDescriptor, Dictionary, Primitive, Value};
use crate::prelude::*;

fn convert_json_value_to_nu_value(v: &serde_hjson::Value) -> Value {
    match v {
        serde_hjson::Value::Null => Value::Primitive(Primitive::String(String::from(""))),
        serde_hjson::Value::Bool(b) => Value::Primitive(Primitive::Boolean(*b)),
        serde_hjson::Value::F64(n) => Value::Primitive(Primitive::Float(OF64::from(*n))),
        serde_hjson::Value::U64(n) => Value::Primitive(Primitive::Int(*n as i64)),
        serde_hjson::Value::I64(n) => Value::Primitive(Primitive::Int(*n as i64)),
        serde_hjson::Value::String(s) => Value::Primitive(Primitive::String(String::from(s))),
        serde_hjson::Value::Array(a) => Value::List(
            a.iter()
                .map(|x| convert_json_value_to_nu_value(x))
                .collect(),
        ),
        serde_hjson::Value::Object(o) => {
            let mut collected = Dictionary::default();
            for (k, v) in o.iter() {
                collected.add(
                    DataDescriptor::from(k.clone()),
                    convert_json_value_to_nu_value(v),
                );
            }
            Value::Object(collected)
        }
    }
}

pub fn from_json_string_to_value(s: String) -> Value {
    let v: serde_hjson::Value = serde_hjson::from_str(&s).unwrap();
    convert_json_value_to_nu_value(&v)
}

pub fn from_json(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let out = args.input;
    Ok(out
        .map(|a| match a {
            Value::Primitive(Primitive::String(s)) => {
                ReturnValue::Value(from_json_string_to_value(s.to_string()))
            }
            _ => ReturnValue::Value(Value::Primitive(Primitive::String(String::from("")))),
        })
        .boxed())
}
