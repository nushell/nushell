use crate::object::base::OF64;
use crate::object::{Primitive, TaggedDictBuilder, Value};
use crate::prelude::*;

fn convert_toml_value_to_nu_value(v: &toml::Value, span: impl Into<Span>) -> Tagged<Value> {
    let span = span.into();

    match v {
        toml::Value::Boolean(b) => Value::Primitive(Primitive::Boolean(*b)).tagged(span),
        toml::Value::Integer(n) => Value::Primitive(Primitive::Int(*n)).tagged(span),
        toml::Value::Float(n) => Value::Primitive(Primitive::Float(OF64::from(*n))).tagged(span),
        toml::Value::String(s) => Value::Primitive(Primitive::String(String::from(s))).tagged(span),
        toml::Value::Array(a) => Value::List(
            a.iter()
                .map(|x| convert_toml_value_to_nu_value(x, span))
                .collect(),
        )
        .tagged(span),
        toml::Value::Datetime(dt) => {
            Value::Primitive(Primitive::String(dt.to_string())).tagged(span)
        }
        toml::Value::Table(t) => {
            let mut collected = TaggedDictBuilder::new(span);

            for (k, v) in t.iter() {
                collected.insert_tagged(k.clone(), convert_toml_value_to_nu_value(v, span));
            }

            collected.into_tagged_value()
        }
    }
}

pub fn from_toml_string_to_value(
    s: String,
    span: impl Into<Span>,
) -> Result<Tagged<Value>, Box<dyn std::error::Error>> {
    let v: toml::Value = s.parse::<toml::Value>()?;
    Ok(convert_toml_value_to_nu_value(&v, span))
}

pub fn from_toml(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let out = args.input;
    let span = args.call_info.name_span;
    Ok(out
        .values
        .map(move |a| {
            let value_span = a.span();
            match a.item {
                Value::Primitive(Primitive::String(s)) => {
                    match from_toml_string_to_value(s, value_span) {
                        Ok(x) => ReturnSuccess::value(x),
                        Err(_) => Err(ShellError::maybe_labeled_error(
                            "Could not parse as TOML",
                            "piped data failed TOML parse",
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
