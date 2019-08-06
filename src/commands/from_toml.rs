use crate::object::base::OF64;
use crate::object::{Primitive, TaggedDictBuilder, Value};
use crate::prelude::*;

fn convert_toml_value_to_nu_value(v: &toml::Value, tag: impl Into<Tag>) -> Tagged<Value> {
    let tag = tag.into();

    match v {
        toml::Value::Boolean(b) => Value::Primitive(Primitive::Boolean(*b)).tagged(tag),
        toml::Value::Integer(n) => Value::Primitive(Primitive::Int(*n)).tagged(tag),
        toml::Value::Float(n) => Value::Primitive(Primitive::Float(OF64::from(*n))).tagged(tag),
        toml::Value::String(s) => Value::Primitive(Primitive::String(String::from(s))).tagged(tag),
        toml::Value::Array(a) => Value::List(
            a.iter()
                .map(|x| convert_toml_value_to_nu_value(x, tag))
                .collect(),
        )
        .tagged(tag),
        toml::Value::Datetime(dt) => {
            Value::Primitive(Primitive::String(dt.to_string())).tagged(tag)
        }
        toml::Value::Table(t) => {
            let mut collected = TaggedDictBuilder::new(tag);

            for (k, v) in t.iter() {
                collected.insert_tagged(k.clone(), convert_toml_value_to_nu_value(v, tag));
            }

            collected.into_tagged_value()
        }
    }
}

pub fn from_toml_string_to_value(
    s: String,
    tag: impl Into<Tag>,
) -> Result<Tagged<Value>, Box<dyn std::error::Error>> {
    let v: toml::Value = s.parse::<toml::Value>()?;
    Ok(convert_toml_value_to_nu_value(&v, tag))
}

pub fn from_toml(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let out = args.input;
    let span = args.call_info.name_span;
    Ok(out
        .values
        .map(move |a| {
            let value_tag = a.tag();
            match a.item {
                Value::Primitive(Primitive::String(s)) => {
                    match from_toml_string_to_value(s, value_tag) {
                        Ok(x) => ReturnSuccess::value(x),
                        Err(_) => Err(ShellError::labeled_error_with_secondary(
                            "Could not parse as TOML",
                            "input cannot be parsed as TOML",
                            span,
                            "value originates from here",
                            value_tag.span,
                        )),
                    }
                }
                x => Err(ShellError::labeled_error_with_secondary(
                    "Expected a string from pipeline",
                    "requires string input",
                    span,
                    format!("{} originates from here", x.type_name()),
                    value_tag.span,
                )),
            }
        })
        .to_output_stream())
}
