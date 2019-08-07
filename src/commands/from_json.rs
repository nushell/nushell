use crate::object::base::OF64;
use crate::object::{Primitive, TaggedDictBuilder, Value};
use crate::prelude::*;

fn convert_json_value_to_nu_value(v: &serde_hjson::Value, tag: impl Into<Tag>) -> Tagged<Value> {
    let tag = tag.into();

    match v {
        serde_hjson::Value::Null => {
            Value::Primitive(Primitive::String(String::from(""))).tagged(tag)
        }
        serde_hjson::Value::Bool(b) => Value::Primitive(Primitive::Boolean(*b)).tagged(tag),
        serde_hjson::Value::F64(n) => {
            Value::Primitive(Primitive::Float(OF64::from(*n))).tagged(tag)
        }
        serde_hjson::Value::U64(n) => Value::Primitive(Primitive::Int(*n as i64)).tagged(tag),
        serde_hjson::Value::I64(n) => Value::Primitive(Primitive::Int(*n as i64)).tagged(tag),
        serde_hjson::Value::String(s) => {
            Value::Primitive(Primitive::String(String::from(s))).tagged(tag)
        }
        serde_hjson::Value::Array(a) => Value::List(
            a.iter()
                .map(|x| convert_json_value_to_nu_value(x, tag))
                .collect(),
        )
        .tagged(tag),
        serde_hjson::Value::Object(o) => {
            let mut collected = TaggedDictBuilder::new(tag);
            for (k, v) in o.iter() {
                collected.insert_tagged(k.clone(), convert_json_value_to_nu_value(v, tag));
            }

            collected.into_tagged_value()
        }
    }
}

pub fn from_json_string_to_value(
    s: String,
    tag: impl Into<Tag>,
) -> serde_hjson::Result<Tagged<Value>> {
    let v: serde_hjson::Value = serde_hjson::from_str(&s)?;
    Ok(convert_json_value_to_nu_value(&v, tag))
}

pub fn from_json(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let out = args.input;
    let span = args.call_info.name_span;
    Ok(out
        .values
        .map(move |a| {
            let value_tag = a.tag();
            match a.item {
                Value::Primitive(Primitive::String(s)) => {
                    match from_json_string_to_value(s, value_tag) {
                        Ok(x) => ReturnSuccess::value(x),
                        Err(_) => Err(ShellError::labeled_error_with_secondary(
                            "Could not parse as JSON",
                            "input cannot be parsed as JSON",
                            span,
                            "value originates from here",
                            value_tag.span,
                        )),
                    }
                }
                _ => Err(ShellError::labeled_error_with_secondary(
                    "Expected a string from pipeline",
                    "requires string input",
                    span,
                    "value originates from here",
                    a.span(),
                )),
            }
        })
        .to_output_stream())
}
