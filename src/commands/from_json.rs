use crate::object::base::OF64;
use crate::object::{Primitive, SpannedDictBuilder, Value};
use crate::prelude::*;

fn convert_json_value_to_nu_value(v: &serde_hjson::Value, span: impl Into<Span>) -> Spanned<Value> {
    let span = span.into();

    match v {
        serde_hjson::Value::Null => {
            Value::Primitive(Primitive::String(String::from(""))).spanned(span)
        }
        serde_hjson::Value::Bool(b) => Value::Primitive(Primitive::Boolean(*b)).spanned(span),
        serde_hjson::Value::F64(n) => {
            Value::Primitive(Primitive::Float(OF64::from(*n))).spanned(span)
        }
        serde_hjson::Value::U64(n) => Value::Primitive(Primitive::Int(*n as i64)).spanned(span),
        serde_hjson::Value::I64(n) => Value::Primitive(Primitive::Int(*n as i64)).spanned(span),
        serde_hjson::Value::String(s) => {
            Value::Primitive(Primitive::String(String::from(s))).spanned(span)
        }
        serde_hjson::Value::Array(a) => Value::List(
            a.iter()
                .map(|x| convert_json_value_to_nu_value(x, span))
                .collect(),
        )
        .spanned(span),
        serde_hjson::Value::Object(o) => {
            let mut collected = SpannedDictBuilder::new(span);
            for (k, v) in o.iter() {
                collected.insert_spanned(k.clone(), convert_json_value_to_nu_value(v, span));
            }

            collected.into_spanned_value()
        }
    }
}

pub fn from_json_string_to_value(
    s: String,
    span: impl Into<Span>,
) -> serde_hjson::Result<Spanned<Value>> {
    let v: serde_hjson::Value = serde_hjson::from_str(&s)?;
    Ok(convert_json_value_to_nu_value(&v, span))
}

pub fn from_json(
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;
    let span = args.name_span();
    let out = args.input;

    Ok(out
        .values
        .map(move |a| match a.item {
            Value::Primitive(Primitive::String(s)) => match from_json_string_to_value(s, span) {
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
