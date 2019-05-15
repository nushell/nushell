use crate::errors::ShellError;
use crate::format::{EntriesView, GenericView};
use crate::object::desc::DataDescriptor;
use ansi_term::Color;
use chrono::{DateTime, Utc};
use chrono_humanize::Humanize;
use std::fmt::Debug;
use std::time::SystemTime;

#[derive(Debug, Clone)]
pub enum Primitive {
    Nothing,
    Int(i64),
    Float(f64),
    Bytes(u128),
    String(String),
    Boolean(bool),
    Date(DateTime<Utc>),
}

impl Primitive {
    crate fn format(&self, field_name: Option<&str>) -> String {
        match self {
            Primitive::Nothing => format!("{}", Color::Black.bold().paint("-")),
            Primitive::Bytes(b) => {
                let byte = byte_unit::Byte::from_bytes(*b);

                if byte.get_bytes() == 0u128 {
                    return Color::Black.bold().paint("Empty".to_string()).to_string();
                }

                let byte = byte.get_appropriate_unit(true);

                match byte.get_unit() {
                    byte_unit::ByteUnit::B => format!("{}", byte.format(0)),
                    _ => format!("{}", byte.format(1)),
                }
            }
            Primitive::Int(i) => format!("{}", i),
            Primitive::Float(f) => format!("{}", f),
            Primitive::String(s) => format!("{}", s),
            Primitive::Boolean(b) => match (b, field_name) {
                (true, None) => format!("Yes"),
                (false, None) => format!("No"),
                (true, Some(s)) => format!("{}", s),
                (false, Some(s)) => format!(""),
            },
            Primitive::Date(d) => format!("{}", d.humanize()),
        }
    }
}

#[derive(Debug)]
pub enum Value {
    Primitive(Primitive),
    Object(crate::object::Dictionary),
    List(Vec<Value>),
    Error(Box<ShellError>),
}

impl Value {
    crate fn to_shell_string(&self) -> String {
        match self {
            Value::Primitive(p) => p.format(None),
            Value::Object(o) => format!("[object Object]"),
            Value::List(l) => format!("[list List]"),
            Value::Error(e) => format!("{}", e),
        }
    }

    crate fn data_descriptors(&self) -> Vec<DataDescriptor> {
        match self {
            Value::Primitive(p) => vec![],
            Value::Object(o) => o.data_descriptors(),
            Value::List(l) => vec![],
            Value::Error(e) => vec![],
        }
    }

    crate fn get_data(&'a self, desc: &DataDescriptor) -> crate::MaybeOwned<'a, Value> {
        match self {
            Value::Primitive(p) => crate::MaybeOwned::Owned(Value::nothing()),
            Value::Object(o) => o.get_data(desc),
            Value::List(l) => crate::MaybeOwned::Owned(Value::nothing()),
            Value::Error(e) => crate::MaybeOwned::Owned(Value::nothing()),
        }
    }

    crate fn copy(&self) -> Value {
        match self {
            Value::Primitive(p) => Value::Primitive(p.clone()),
            Value::Object(o) => Value::Object(o.copy_dict()),
            Value::List(l) => {
                let list = l.iter().map(|i| i.copy()).collect();
                Value::List(list)
            }
            Value::Error(e) => Value::Error(Box::new(e.copy_error())),
        }
    }

    crate fn format_leaf(&self, field_name: Option<&str>) -> String {
        match self {
            Value::Primitive(p) => p.format(field_name),
            Value::Object(o) => format!("[object Object]"),
            Value::List(l) => format!("[list List]"),
            Value::Error(e) => format!("{}", e),
        }
    }

    crate fn as_string(&self) -> Result<String, ShellError> {
        match self {
            Value::Primitive(Primitive::String(s)) => Ok(s.to_string()),

            // TODO: this should definitely be more general with better errors
            other => Err(ShellError::string(format!(
                "Expected string, got {:?}",
                other
            ))),
        }
    }

    crate fn as_int(&self) -> Result<i64, ShellError> {
        match self {
            Value::Primitive(Primitive::Int(i)) => Ok(*i),
            // TODO: this should definitely be more general with better errors
            other => Err(ShellError::string(format!(
                "Expected integer, got {:?}",
                other
            ))),
        }
    }

    crate fn string(s: impl Into<String>) -> Value {
        Value::Primitive(Primitive::String(s.into()))
    }

    crate fn bytes(s: impl Into<u128>) -> Value {
        Value::Primitive(Primitive::Bytes(s.into()))
    }

    crate fn int(s: impl Into<i64>) -> Value {
        Value::Primitive(Primitive::Int(s.into()))
    }

    crate fn system_date(s: SystemTime) -> Value {
        Value::Primitive(Primitive::Date(s.into()))
    }

    crate fn system_date_result(s: Result<SystemTime, std::io::Error>) -> Value {
        match s {
            Ok(time) => Value::Primitive(Primitive::Date(time.into())),
            Err(err) => Value::Error(Box::new(ShellError::string(format!("{}", err)))),
        }
    }

    crate fn boolean(s: impl Into<bool>) -> Value {
        Value::Primitive(Primitive::Boolean(s.into()))
    }

    crate fn nothing() -> Value {
        Value::Primitive(Primitive::Nothing)
    }

    crate fn list(values: impl Into<Vec<Value>>) -> Value {
        Value::List(values.into())
    }
}

crate fn select(obj: &Value, fields: &[String]) -> crate::object::Dictionary {
    let mut out = crate::object::Dictionary::default();

    let descs = obj.data_descriptors();

    for field in fields {
        match descs.iter().find(|d| d.name == *field) {
            None => out.add(field.to_string(), Value::nothing()),
            Some(desc) => out.add(field.to_string(), obj.get_data(desc).borrow().copy()),
        }
    }

    out
}

crate fn reject(obj: &Value, fields: &[String]) -> crate::object::Dictionary {
    let mut out = crate::object::Dictionary::default();

    let descs = obj.data_descriptors();

    for desc in descs {
        if fields.contains(&desc.name) {
            continue;
        } else {
            out.add(desc.name.clone(), obj.get_data(&desc).borrow().copy())
        }
    }

    out
}
