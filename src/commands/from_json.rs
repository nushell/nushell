use crate::object::base::OF64;
use crate::object::{Dictionary, Primitive, Value};
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
                .map(|x| convert_json_value_to_nu_value(x).spanned_unknown())
                .collect(),
        ),
        serde_hjson::Value::Object(o) => {
            let mut collected = Dictionary::default();
            for (k, v) in o.iter() {
                collected.add(k.clone(), convert_json_value_to_nu_value(v));
            }
            Value::Object(collected)
        }
    }
}

pub fn from_json_string_to_value(s: String) -> serde_hjson::Result<Value> {
    let v: serde_hjson::Value = serde_hjson::from_str(&s)?;
    Ok(convert_json_value_to_nu_value(&v))
}

pub fn from_json(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let out = args.input;
    let span = args.name_span;
    Ok(out
        .values
        .map(move |a| match a.item {
            Value::Primitive(Primitive::String(s)) => match from_json_string_to_value(s) {
                Ok(x) => ReturnSuccess::value(x.spanned(a.span)),
                Err(_) => Err(ShellError::maybe_labeled_error(
                    "Could not parse as JSON",
                    "piped data failed JSON parse",
                    span,
                )),
            },
            _ => Err(ShellError::maybe_labeled_error(
                "Expected string values from pipeline",
                "expects strings from pipeline",
                span,
            )),
        })
        .to_output_stream())
}
