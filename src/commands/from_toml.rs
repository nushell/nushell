use crate::object::base::OF64;
use crate::object::{Dictionary, Primitive, SpannedDictBuilder, Value};
use crate::prelude::*;

fn convert_toml_value_to_nu_value(v: &toml::Value, span: impl Into<Span>) -> Spanned<Value> {
    let span = span.into();

    match v {
        toml::Value::Boolean(b) => Value::Primitive(Primitive::Boolean(*b)).spanned(span),
        toml::Value::Integer(n) => Value::Primitive(Primitive::Int(*n)).spanned(span),
        toml::Value::Float(n) => Value::Primitive(Primitive::Float(OF64::from(*n))).spanned(span),
        toml::Value::String(s) => {
            Value::Primitive(Primitive::String(String::from(s))).spanned(span)
        }
        toml::Value::Array(a) => Value::List(
            a.iter()
                .map(|x| convert_toml_value_to_nu_value(x, span))
                .collect(),
        )
        .spanned(span),
        toml::Value::Datetime(dt) => {
            Value::Primitive(Primitive::String(dt.to_string())).spanned(span)
        }
        toml::Value::Table(t) => {
            let mut collected = SpannedDictBuilder::new(span);

            for (k, v) in t.iter() {
                collected.insert_spanned(k.clone(), convert_toml_value_to_nu_value(v, span));
            }

            collected.into_spanned_value()
        }
    }
}

pub fn from_toml_string_to_value(
    s: String,
    span: impl Into<Span>,
) -> Result<Spanned<Value>, Box<dyn std::error::Error>> {
    let v: toml::Value = s.parse::<toml::Value>()?;
    Ok(convert_toml_value_to_nu_value(&v, span))
}

pub fn from_toml(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let out = args.input;
    let span = args.name_span;
    Ok(out
        .values
        .map(move |a| match a.item {
            Value::Primitive(Primitive::String(s)) => match from_toml_string_to_value(s, span) {
                Ok(x) => ReturnSuccess::value(x.spanned(a.span)),
                Err(_) => Err(ShellError::maybe_labeled_error(
                    "Could not parse as TOML",
                    "piped data failed TOML parse",
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
