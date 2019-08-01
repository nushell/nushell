use crate::object::{Primitive, TaggedDictBuilder, Value};
use crate::prelude::*;
use std::collections::HashMap;

fn convert_ini_second_to_nu_value(
    v: &HashMap<String, String>,
    span: impl Into<Span>,
) -> Tagged<Value> {
    let mut second = TaggedDictBuilder::new(span);

    for (key, value) in v.into_iter() {
        second.insert(key.clone(), Primitive::String(value.clone()));
    }

    second.into_tagged_value()
}

fn convert_ini_top_to_nu_value(
    v: &HashMap<String, HashMap<String, String>>,
    span: impl Into<Span>,
) -> Tagged<Value> {
    let span = span.into();
    let mut top_level = TaggedDictBuilder::new(span);

    for (key, value) in v.iter() {
        top_level.insert_tagged(key.clone(), convert_ini_second_to_nu_value(value, span));
    }

    top_level.into_tagged_value()
}

pub fn from_ini_string_to_value(
    s: String,
    span: impl Into<Span>,
) -> Result<Tagged<Value>, Box<dyn std::error::Error>> {
    let v: HashMap<String, HashMap<String, String>> = serde_ini::from_str(&s)?;
    Ok(convert_ini_top_to_nu_value(&v, span))
}

pub fn from_ini(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let out = args.input;
    let span = args.call_info.name_span;
    Ok(out
        .values
        .map(move |a| {
            let value_span = a.span();
            match a.item {
                Value::Primitive(Primitive::String(s)) => {
                    match from_ini_string_to_value(s, value_span) {
                        Ok(x) => ReturnSuccess::value(x),
                        Err(_) => Err(ShellError::maybe_labeled_error(
                            "Could not parse as INI",
                            "piped data failed INI parse",
                            span,
                        )),
                    }
                }
                _ => Err(ShellError::maybe_labeled_error(
                    "Expected string values from pipeline",
                    "expects strings from pipeline",
                    span,
                )),
            }
        })
        .to_output_stream())
}
