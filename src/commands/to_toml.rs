use crate::object::{Primitive, Value};
use crate::prelude::*;

pub fn value_to_toml_value(v: &Value) -> toml::Value {
    match v {
        Value::Primitive(Primitive::Boolean(b)) => toml::Value::Boolean(*b),
        Value::Primitive(Primitive::Bytes(b)) => toml::Value::Integer(*b as i64),
        Value::Primitive(Primitive::Date(d)) => toml::Value::String(d.to_string()),
        Value::Primitive(Primitive::EndOfStream) => {
            toml::Value::String("<End of Stream>".to_string())
        }
        Value::Primitive(Primitive::BeginningOfStream) => {
            toml::Value::String("<Beginning of Stream>".to_string())
        }
        Value::Primitive(Primitive::Float(f)) => toml::Value::Float(f.into_inner()),
        Value::Primitive(Primitive::Int(i)) => toml::Value::Integer(*i),
        Value::Primitive(Primitive::Nothing) => toml::Value::String("<Nothing>".to_string()),
        Value::Primitive(Primitive::String(s)) => toml::Value::String(s.clone()),
        Value::Primitive(Primitive::Path(s)) => toml::Value::String(s.display().to_string()),

        Value::List(l) => toml::Value::Array(l.iter().map(|x| value_to_toml_value(x)).collect()),
        Value::Block(_) => toml::Value::String("<Block>".to_string()),
        Value::Binary(b) => {
            toml::Value::Array(b.iter().map(|x| toml::Value::Integer(*x as i64)).collect())
        }
        Value::Object(o) => {
            let mut m = toml::map::Map::new();
            for (k, v) in o.entries.iter() {
                m.insert(k.clone(), value_to_toml_value(v));
            }
            toml::Value::Table(m)
        }
    }
}

pub fn to_toml(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let out = args.input;
    let name_span = args.call_info.name_span;

    Ok(out
        .values
        .map(move |a| match toml::to_string(&value_to_toml_value(&a)) {
            Ok(val) => {
                return ReturnSuccess::value(
                    Value::Primitive(Primitive::String(val)).simple_spanned(name_span),
                )
            }
            _ => Err(ShellError::labeled_error_with_secondary(
                "Expected an object with TOML-compatible structure from pipeline",
                "requires TOML-compatible input",
                name_span,
                format!("{} originates from here", a.item.type_name()),
                a.span(),
            )),
        })
        .to_output_stream())
}
