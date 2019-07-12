use crate::object::{Dictionary, Primitive, SpannedDictBuilder, Value};
use crate::prelude::*;
use std::collections::HashMap;

fn convert_ini_second_to_nu_value(
    v: &HashMap<String, String>,
    span: impl Into<Span>,
) -> Spanned<Value> {
    let mut second = SpannedDictBuilder::new(span);

    for (key, value) in v.into_iter() {
        second.insert(key.clone(), Primitive::String(value.clone()));
    }

    second.into_spanned_value()
}

fn convert_ini_top_to_nu_value(
    v: &HashMap<String, HashMap<String, String>>,
    span: impl Into<Span>,
) -> Spanned<Value> {
    let span = span.into();
    let mut top_level = SpannedDictBuilder::new(span);

    for (key, value) in v.iter() {
        top_level.insert_spanned(key.clone(), convert_ini_second_to_nu_value(value, span));
    }

    top_level.into_spanned_value()
}

pub fn from_ini_string_to_value(
    s: String,
    span: impl Into<Span>,
) -> Result<Spanned<Value>, Box<dyn std::error::Error>> {
    let v: HashMap<String, HashMap<String, String>> = serde_ini::from_str(&s)?;
    Ok(convert_ini_top_to_nu_value(&v, span))
}

pub fn from_ini(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let out = args.input;
    let span = args.name_span;
    Ok(out
        .values
        .map(move |a| match a.item {
            Value::Primitive(Primitive::String(s)) => match from_ini_string_to_value(s, span) {
                Ok(x) => ReturnSuccess::value(x.spanned(a.span)),
                Err(e) => Err(ShellError::maybe_labeled_error(
                    "Could not parse as INI",
                    format!("{:#?}", e),
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
