use crate::object::base::OF64;
use crate::object::{Primitive, TaggedDictBuilder, Value};
use crate::prelude::*;

fn convert_yaml_value_to_nu_value(v: &serde_yaml::Value, tag: impl Into<Tag>) -> Tagged<Value> {
    let tag = tag.into();

    match v {
        serde_yaml::Value::Bool(b) => Value::Primitive(Primitive::Boolean(*b)).tagged(tag),
        serde_yaml::Value::Number(n) if n.is_i64() => {
            Value::Primitive(Primitive::Int(n.as_i64().unwrap())).tagged(tag)
        }
        serde_yaml::Value::Number(n) if n.is_f64() => {
            Value::Primitive(Primitive::Float(OF64::from(n.as_f64().unwrap()))).tagged(tag)
        }
        serde_yaml::Value::String(s) => Value::string(s).tagged(tag),
        serde_yaml::Value::Sequence(a) => Value::List(
            a.iter()
                .map(|x| convert_yaml_value_to_nu_value(x, tag))
                .collect(),
        )
        .tagged(tag),
        serde_yaml::Value::Mapping(t) => {
            let mut collected = TaggedDictBuilder::new(tag);

            for (k, v) in t.iter() {
                match k {
                    serde_yaml::Value::String(k) => {
                        collected.insert_tagged(k.clone(), convert_yaml_value_to_nu_value(v, tag));
                    }
                    _ => unimplemented!("Unknown key type"),
                }
            }

            collected.into_tagged_value()
        }
        serde_yaml::Value::Null => Value::Primitive(Primitive::Nothing).tagged(tag),
        x => unimplemented!("Unsupported yaml case: {:?}", x),
    }
}

pub fn from_yaml_string_to_value(
    s: String,
    tag: impl Into<Tag>,
) -> serde_yaml::Result<Tagged<Value>> {
    let v: serde_yaml::Value = serde_yaml::from_str(&s)?;
    Ok(convert_yaml_value_to_nu_value(&v, tag))
}

pub fn from_yaml(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let out = args.input;
    let span = args.call_info.name_span;
    Ok(out
        .values
        .map(move |a| {
            let value_tag = a.tag();
            match a.item {
                Value::Primitive(Primitive::String(s)) => {
                    match from_yaml_string_to_value(s, value_tag) {
                        Ok(x) => ReturnSuccess::value(x),
                        Err(_) => Err(ShellError::labeled_error_with_secondary(
                            "Could not parse as YAML",
                            "input cannot be parsed as YAML",
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
