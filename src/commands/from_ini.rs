use crate::object::{Primitive, TaggedDictBuilder, Value};
use crate::prelude::*;
use std::collections::HashMap;

fn convert_ini_second_to_nu_value(
    v: &HashMap<String, String>,
    tag: impl Into<Tag>,
) -> Tagged<Value> {
    let mut second = TaggedDictBuilder::new(tag);

    for (key, value) in v.into_iter() {
        second.insert(key.clone(), Primitive::String(value.clone()));
    }

    second.into_tagged_value()
}

fn convert_ini_top_to_nu_value(
    v: &HashMap<String, HashMap<String, String>>,
    tag: impl Into<Tag>,
) -> Tagged<Value> {
    let tag = tag.into();
    let mut top_level = TaggedDictBuilder::new(tag);

    for (key, value) in v.iter() {
        top_level.insert_tagged(key.clone(), convert_ini_second_to_nu_value(value, tag));
    }

    top_level.into_tagged_value()
}

pub fn from_ini_string_to_value(
    s: String,
    tag: impl Into<Tag>,
) -> Result<Tagged<Value>, Box<dyn std::error::Error>> {
    let v: HashMap<String, HashMap<String, String>> = serde_ini::from_str(&s)?;
    Ok(convert_ini_top_to_nu_value(&v, tag))
}

pub fn from_ini(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let out = args.input;
    let span = args.call_info.name_span;
    Ok(out
        .values
        .map(move |a| {
            let value_tag = a.tag();
            match a.item {
                Value::Primitive(Primitive::String(s)) => {
                    match from_ini_string_to_value(s, value_tag) {
                        Ok(x) => ReturnSuccess::value(x),
                        Err(_) => Err(ShellError::labeled_error_with_secondary(
                            "Could not parse as INI",
                            "input cannot be parsed as INI",
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
