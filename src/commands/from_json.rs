use crate::object::base::OF64;
use crate::object::{Primitive, TaggedDictBuilder, Value};
use crate::prelude::*;

fn convert_json_value_to_nu_value(v: &serde_hjson::Value, span: impl Into<Span>) -> Tagged<Value> {
    let span = span.into();

    match v {
        serde_hjson::Value::Null => {
            Value::Primitive(Primitive::String(String::from(""))).tagged(span)
        }
        serde_hjson::Value::Bool(b) => Value::Primitive(Primitive::Boolean(*b)).tagged(span),
        serde_hjson::Value::F64(n) => {
            Value::Primitive(Primitive::Float(OF64::from(*n))).tagged(span)
        }
        serde_hjson::Value::U64(n) => Value::Primitive(Primitive::Int(*n as i64)).tagged(span),
        serde_hjson::Value::I64(n) => Value::Primitive(Primitive::Int(*n as i64)).tagged(span),
        serde_hjson::Value::String(s) => {
            Value::Primitive(Primitive::String(String::from(s))).tagged(span)
        }
        serde_hjson::Value::Array(a) => Value::List(
            a.iter()
                .map(|x| convert_json_value_to_nu_value(x, span))
                .collect(),
        )
        .tagged(span),
        serde_hjson::Value::Object(o) => {
            let mut collected = TaggedDictBuilder::new(span);
            for (k, v) in o.iter() {
                collected.insert_tagged(k.clone(), convert_json_value_to_nu_value(v, span));
            }

            collected.into_tagged_value()
        }
    }
}

pub fn from_json_string_to_value(
    s: String,
    span: impl Into<Span>,
) -> serde_hjson::Result<Tagged<Value>> {
    let v: serde_hjson::Value = serde_hjson::from_str(&s)?;
    Ok(convert_json_value_to_nu_value(&v, span))
}

pub fn from_json(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let out = args.input;
    let span = args.call_info.name_span;
    Ok(out
        .values
        .map(move |a| {
            let value_span = a.span();
            match a.item {
                Value::Primitive(Primitive::String(s)) => {
                    match from_json_string_to_value(s, value_span) {
                        Ok(x) => ReturnSuccess::value(x),
                        Err(_) => Err(ShellError::maybe_labeled_error(
                            "Could not parse as JSON",
                            "piped data failed JSON parse",
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
