use crate::object::{DataDescriptor, Dictionary, Primitive, Value};
use crate::prelude::*;
use indexmap::IndexMap;
use std::collections::HashMap;

fn convert_ini_second_to_nu_value(v: &HashMap<String, String>) -> Value {
    let mut second = Dictionary::new(IndexMap::new());
    for (key, value) in v.into_iter() {
        second.add(
            DataDescriptor::from(key.as_str()),
            Value::Primitive(Primitive::String(value.clone())),
        );
    }
    Value::Object(second)
}
fn convert_ini_top_to_nu_value(v: &HashMap<String, HashMap<String, String>>) -> Value {
    let mut top_level = Dictionary::new(IndexMap::new());
    for (key, value) in v.iter() {
        top_level.add(
            DataDescriptor::from(key.as_str()),
            convert_ini_second_to_nu_value(value),
        );
    }
    Value::Object(top_level)
}

pub fn from_ini_string_to_value(s: String) -> Result<Value, Box<dyn std::error::Error>> {
    let v: HashMap<String, HashMap<String, String>> = serde_ini::from_str(&s)?;
    Ok(convert_ini_top_to_nu_value(&v))
}

pub fn from_ini(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let out = args.input;
    let span = args.name_span;
    Ok(out
        .map(move |a| match a {
            Value::Primitive(Primitive::String(s)) => match from_ini_string_to_value(s) {
                Ok(x) => ReturnValue::Value(x),
                Err(e) => {
                    ReturnValue::Value(Value::Error(Box::new(ShellError::maybe_labeled_error(
                        "Could not parse as INI",
                        format!("{:#?}", e),
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
