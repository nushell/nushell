use crate::object::{Primitive, Value};
use crate::prelude::*;

pub fn value_to_yaml_value(v: &Value) -> serde_yaml::Value {
    match v {
        Value::Primitive(Primitive::Boolean(b)) => serde_yaml::Value::Bool(*b),
        Value::Primitive(Primitive::Bytes(b)) => {
            serde_yaml::Value::Number(serde_yaml::Number::from(*b as u64))
        }
        Value::Primitive(Primitive::Date(d)) => serde_yaml::Value::String(d.to_string()),
        Value::Primitive(Primitive::EndOfStream) => serde_yaml::Value::Null,
        Value::Primitive(Primitive::Float(f)) => {
            serde_yaml::Value::Number(serde_yaml::Number::from(f.into_inner()))
        }
        Value::Primitive(Primitive::Int(i)) => {
            serde_yaml::Value::Number(serde_yaml::Number::from(*i))
        }
        Value::Primitive(Primitive::Nothing) => serde_yaml::Value::Null,
        Value::Primitive(Primitive::String(s)) => serde_yaml::Value::String(s.clone()),
        Value::Primitive(Primitive::Path(s)) => serde_yaml::Value::String(s.display().to_string()),

        Value::Filesystem => serde_yaml::Value::Null,
        Value::List(l) => {
            serde_yaml::Value::Sequence(l.iter().map(|x| value_to_yaml_value(x)).collect())
        }
        Value::Block(_) => serde_yaml::Value::Null,
        Value::Binary(b) => serde_yaml::Value::Sequence(
            b.iter()
                .map(|x| serde_yaml::Value::Number(serde_yaml::Number::from(*x)))
                .collect(),
        ),
        Value::Object(o) => {
            let mut m = serde_yaml::Mapping::new();
            for (k, v) in o.entries.iter() {
                m.insert(serde_yaml::Value::String(k.clone()), value_to_yaml_value(v));
            }
            serde_yaml::Value::Mapping(m)
        }
    }
}

pub fn to_yaml(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let out = args.input;
    let name_span = args.name_span;
    Ok(out
        .values
        .map(
            move |a| match serde_yaml::to_string(&value_to_yaml_value(&a)) {
                Ok(x) => {
                    ReturnSuccess::value(Value::Primitive(Primitive::String(x)).spanned(name_span))
                }
                Err(_) => Err(ShellError::maybe_labeled_error(
                    "Can not convert to YAML string",
                    "can not convert piped data to YAML string",
                    name_span,
                )),
            },
        )
        .to_output_stream())
}
