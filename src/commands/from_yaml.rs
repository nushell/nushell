use crate::object::base::OF64;
use crate::object::{Primitive, TaggedDictBuilder, Value};
use crate::prelude::*;

fn convert_yaml_value_to_nu_value(v: &serde_yaml::Value, span: impl Into<Span>) -> Tagged<Value> {
    let span = span.into();

    match v {
        serde_yaml::Value::Bool(b) => Value::Primitive(Primitive::Boolean(*b)).tagged(span),
        serde_yaml::Value::Number(n) if n.is_i64() => {
            Value::Primitive(Primitive::Int(n.as_i64().unwrap())).tagged(span)
        }
        serde_yaml::Value::Number(n) if n.is_f64() => {
            Value::Primitive(Primitive::Float(OF64::from(n.as_f64().unwrap()))).tagged(span)
        }
        serde_yaml::Value::String(s) => Value::string(s).tagged(span),
        serde_yaml::Value::Sequence(a) => Value::List(
            a.iter()
                .map(|x| convert_yaml_value_to_nu_value(x, span))
                .collect(),
        )
        .tagged(span),
        serde_yaml::Value::Mapping(t) => {
            let mut collected = TaggedDictBuilder::new(span);

            for (k, v) in t.iter() {
                match k {
                    serde_yaml::Value::String(k) => {
                        collected.insert_tagged(k.clone(), convert_yaml_value_to_nu_value(v, span));
                    }
                    _ => unimplemented!("Unknown key type"),
                }
            }

            collected.into_tagged_value()
        }
        serde_yaml::Value::Null => Value::Primitive(Primitive::Nothing).tagged(span),
        x => unimplemented!("Unsupported yaml case: {:?}", x),
    }
}

pub fn from_yaml_string_to_value(
    s: String,
    span: impl Into<Span>,
) -> serde_yaml::Result<Tagged<Value>> {
    let v: serde_yaml::Value = serde_yaml::from_str(&s)?;
    Ok(convert_yaml_value_to_nu_value(&v, span))
}

pub fn from_yaml(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let out = args.input;
    let span = args.call_info.name_span;
    Ok(out
        .values
        .map(move |a| {
            let value_span = a.span();
            match a.item {
                Value::Primitive(Primitive::String(s)) => {
                    match from_yaml_string_to_value(s, value_span) {
                        Ok(x) => ReturnSuccess::value(x),
                        Err(_) => Err(ShellError::maybe_labeled_error(
                            "Could not parse as YAML",
                            "piped data failed YAML parse",
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
