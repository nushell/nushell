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
        .map(move |a| {
            match toml::to_string(&value_to_toml_value(&a)) {
                Ok(val) => {
                    return ReturnSuccess::value(
                        Value::Primitive(Primitive::String(val)).spanned(name_span),
                    )
                }

                Err(err) => Err(ShellError::type_error(
                    "serializable to toml",
                    format!("{:?} - {:?}", a.type_name(), err).spanned(name_span),
                )), // toml::Value::String(String) => {
                    //     return ReturnSuccess::value(
                    //         Value::Primitive(Primitive::String(x)).spanned(name_span),
                    //     )
                    // }
                    // toml::Value::Integer(i64) => "Integer",
                    // toml::Value::Float(f64) => "Decimal",
                    // toml::Value::Boolean(bool) => "Boolean",
                    // toml::Value::Datetime(Datetime) => "Date",
                    // toml::Value::Array(Array) => "Array",
                    // toml::Value::Table(Table) => "Table",
            }
            // return Err(ShellError::type_error("String", ty.spanned(name_span)));

            // Err(_) => Err(ShellError::maybe_labeled_error(
            //     "Can not convert to TOML string",
            //     "can not convert piped data to TOML string",
            //     name_span,
            // )),
        })
        .to_output_stream())
}
