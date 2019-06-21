use crate::object::base::OF64;
use crate::object::{DataDescriptor, Dictionary, Primitive, Value};
use crate::prelude::*;

fn convert_yaml_value_to_nu_value(v: &serde_yaml::Value) -> Value {
    match v {
        serde_yaml::Value::Bool(b) => Value::Primitive(Primitive::Boolean(*b)),
        serde_yaml::Value::Number(n) if n.is_i64() => {
            Value::Primitive(Primitive::Int(n.as_i64().unwrap()))
        }
        serde_yaml::Value::Number(n) if n.is_f64() => {
            Value::Primitive(Primitive::Float(OF64::from(n.as_f64().unwrap())))
        }
        serde_yaml::Value::String(s) => Value::Primitive(Primitive::String(s.clone())),
        serde_yaml::Value::Sequence(a) => Value::List(
            a.iter()
                .map(|x| convert_yaml_value_to_nu_value(x))
                .collect(),
        ),
        serde_yaml::Value::Mapping(t) => {
            let mut collected = Dictionary::default();
            for (k, v) in t.iter() {
                match k {
                    serde_yaml::Value::String(k) => {
                        collected.add(
                            DataDescriptor::from(k.clone()),
                            convert_yaml_value_to_nu_value(v),
                        );
                    }
                    _ => unimplemented!("Unknown key type"),
                }
            }
            Value::Object(collected)
        }
        _ => unimplemented!("Unsupported yaml case"),
    }
}

pub fn from_yaml_string_to_value(s: String) -> serde_yaml::Result<Value> {
    let v: serde_yaml::Value = serde_yaml::from_str(&s)?;
    Ok(convert_yaml_value_to_nu_value(&v))
}

pub fn from_yaml(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let out = args.input;
    let span = args.name_span;
    Ok(out
        .map(move |a| match a {
            Value::Primitive(Primitive::String(s)) => match from_yaml_string_to_value(s) {
                Ok(x) => ReturnValue::Value(x),
                Err(_) => {
                    ReturnValue::Value(Value::Error(Box::new(ShellError::maybe_labeled_error(
                        "Could not parse as YAML",
                        "piped data failed YAML parse",
                        span,
                    ))))
                }
            },
            _ => ReturnValue::Value(Value::Error(Box::new(ShellError::maybe_labeled_error(
                "Expected string values from pipeline",
                "expects strings from pipeline",
                span,
            )))),
        })
        .boxed())
}
