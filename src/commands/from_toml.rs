use crate::object::{Primitive, Value, Dictionary, DataDescriptor};
use crate::object::base::OF64;
use crate::prelude::*;

fn convert_toml_value_to_nu_value(v: &toml::Value) -> Value {
    match v {
        toml::Value::Boolean(b) => Value::Primitive(Primitive::Boolean(*b)),
        toml::Value::Integer(n) => Value::Primitive(Primitive::Int(*n)),
        toml::Value::Float(n) => Value::Primitive(Primitive::Float(OF64::from(*n))),
        toml::Value::String(s) => Value::Primitive(Primitive::String(s.clone())),
        toml::Value::Array(a) => Value::List(a.iter().map(|x| convert_toml_value_to_nu_value(x)).collect()),
        toml::Value::Datetime(dt) => Value::Primitive(Primitive::String(dt.to_string())),
        toml::Value::Table(t) => {
            let mut collected = Dictionary::default();
            for (k, v) in t.iter() {
                collected.add(DataDescriptor::from(k.clone()), convert_toml_value_to_nu_value(v));
            }
            Value::Object(collected)
        }
    }
}

pub fn from_toml_string_to_value(s: String) -> Value {
    let v: toml::Value = s.parse::<toml::Value>().unwrap();
    convert_toml_value_to_nu_value(&v)
}

pub fn from_toml(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let out = args.input;
    Ok(out
        .map(|a| match a {
            Value::Primitive(Primitive::String(s)) => {
                ReturnValue::Value(from_toml_string_to_value(s))
            }
            _ => ReturnValue::Value(Value::Primitive(Primitive::String("".to_string()))),
        })
        .boxed())
}
