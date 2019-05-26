use crate::errors::ShellError;
use crate::object::DataDescriptor;
use crate::parser::tokens::{self, Operator};
use crate::prelude::*;
use ansi_term::Color;
use chrono::{DateTime, Utc};
use chrono_humanize::Humanize;
use derive_new::new;
use ordered_float::OrderedFloat;
use std::time::SystemTime;

type OF64 = OrderedFloat<f64>;

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum Primitive {
    Nothing,
    Int(i64),
    #[allow(unused)]
    Float(OF64),
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
            Primitive::Float(f) => format!("{:.*}", 2, f.into_inner()),
            Primitive::String(s) => format!("{}", s),
            Primitive::Boolean(b) => match (b, field_name) {
                (true, None) => format!("Yes"),
                (false, None) => format!("No"),
                (true, Some(s)) => format!("{}", s),
                (false, Some(_)) => format!(""),
            },
            Primitive::Date(d) => format!("{}", d.humanize()),
        }
    }
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone, new)]
pub struct Operation {
    crate left: Value,
    crate operator: Operator,
    crate right: Value,
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone)]
pub enum Value {
    Primitive(Primitive),
    Object(crate::object::Dictionary),
    List(Vec<Value>),
    Operation(Box<Operation>),

    #[allow(unused)]
    Error(Box<ShellError>),
}

impl Value {
    crate fn from_leaf(leaf: &tokens::Leaf) -> Value {
        use tokens::*;

        match leaf {
            Leaf::String(s) => Value::string(s),
            Leaf::Bare(s) => Value::string(s),
            Leaf::Boolean(b) => Value::boolean(*b),
            Leaf::Int(i) => Value::int(*i),
        }
    }

    crate fn from_expr(expr: &tokens::Expression) -> Value {
        use tokens::*;

        match expr {
            Expression::Leaf(leaf) => Value::from_leaf(leaf),

            Expression::Binary(Binary {
                left,
                operator,
                right,
            }) => Value::Operation(Box::new(Operation::new(
                Value::from_leaf(left),
                *operator,
                Value::from_leaf(right),
            ))),
        }
    }

    crate fn data_descriptors(&self) -> Vec<DataDescriptor> {
        match self {
            Value::Primitive(_) => vec![DataDescriptor::value_of()],
            Value::Object(o) => o.data_descriptors(),
            Value::List(_) => vec![],
            Value::Operation(_) => vec![],
            Value::Error(_) => vec![],
        }
    }

    crate fn get_data_by_key(&'a self, name: &str) -> MaybeOwned<'a, Value> {
        match self {
            Value::Primitive(_) => MaybeOwned::Owned(Value::nothing()),
            Value::Object(o) => o.get_data_by_key(name),
            Value::List(_) => MaybeOwned::Owned(Value::nothing()),
            Value::Operation(_) => MaybeOwned::Owned(Value::nothing()),
            Value::Error(_) => MaybeOwned::Owned(Value::nothing()),
        }
    }

    crate fn get_data(&'a self, desc: &DataDescriptor) -> MaybeOwned<'a, Value> {
        match self {
            p @ Value::Primitive(_) => MaybeOwned::Borrowed(p),
            Value::Object(o) => o.get_data(desc),
            Value::List(_) => MaybeOwned::Owned(Value::nothing()),
            Value::Operation(_) => MaybeOwned::Owned(Value::nothing()),
            Value::Error(_) => MaybeOwned::Owned(Value::nothing()),
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
            Value::Operation(o) => Value::Operation(o.clone()),
            Value::Error(e) => Value::Error(Box::new(e.copy_error())),
        }
    }

    crate fn format_leaf(&self, field_name: Option<&str>) -> String {
        match self {
            Value::Primitive(p) => p.format(field_name),
            Value::Object(_) => format!("[object Object]"),
            Value::List(_) => format!("[list List]"),
            Value::Operation(_) => format!("[operation Operation]"),
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

    crate fn as_operation(&self) -> Result<Operation, ShellError> {
        match self {
            Value::Operation(o) => Ok(*o.clone()),

            // TODO: this should definitely be more general with better errors
            other => Err(ShellError::string(format!(
                "Expected operation, got {:?}",
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

    #[allow(unused)]
    crate fn as_bool(&self) -> Result<bool, ShellError> {
        match self {
            Value::Primitive(Primitive::Boolean(b)) => Ok(*b),
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

    crate fn float(s: impl Into<OF64>) -> Value {
        Value::Primitive(Primitive::Float(s.into()))
    }

    #[allow(unused)]
    crate fn bool(s: impl Into<bool>) -> Value {
        Value::Primitive(Primitive::Boolean(s.into()))
    }

    crate fn system_date(s: SystemTime) -> Value {
        Value::Primitive(Primitive::Date(s.into()))
    }

    #[allow(unused)]
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

    #[allow(unused)]
    crate fn list(values: impl Into<Vec<Value>>) -> Value {
        Value::List(values.into())
    }
}

crate fn select_fields(obj: &Value, fields: &[String]) -> crate::object::Dictionary {
    let mut out = crate::object::Dictionary::default();

    let descs = obj.data_descriptors();

    for field in fields {
        match descs.iter().find(|d| d.name.is_string(field)) {
            None => out.add(DataDescriptor::for_string_name(field), Value::nothing()),
            Some(desc) => out.add(desc.copy(), obj.get_data(desc).borrow().copy()),
        }
    }

    out
}

crate fn reject_fields(obj: &Value, fields: &[String]) -> crate::object::Dictionary {
    let mut out = crate::object::Dictionary::default();

    let descs = obj.data_descriptors();

    for desc in descs {
        match desc.name.as_string() {
            None => continue,
            Some(s) if fields.iter().any(|field| field == s) => continue,
            Some(_) => out.add(desc.copy(), obj.get_data(&desc).borrow().copy()),
        }
    }

    out
}

crate fn find(obj: &Value, field: &str, op: &Operator, rhs: &Value) -> bool {
    let descs = obj.data_descriptors();
    match descs.iter().find(|d| d.name.is_string(field)) {
        None => false,
        Some(desc) => {
            let v = obj.get_data(desc).borrow().copy();
            //println!("'{:?}' '{:?}' '{:?}'", v, op, rhs);

            match v {
                Value::Primitive(Primitive::Boolean(b)) => match (op, rhs) {
                    (Operator::Equal, Value::Primitive(Primitive::Boolean(b2))) => b == *b2,
                    (Operator::NotEqual, Value::Primitive(Primitive::Boolean(b2))) => b != *b2,
                    _ => false,
                },
                Value::Primitive(Primitive::Bytes(i)) => match (op, rhs) {
                    (Operator::LessThan, Value::Primitive(Primitive::Int(i2))) => i < (*i2 as u128),
                    (Operator::GreaterThan, Value::Primitive(Primitive::Int(i2))) => {
                        i > (*i2 as u128)
                    }
                    (Operator::LessThanOrEqual, Value::Primitive(Primitive::Int(i2))) => {
                        i <= (*i2 as u128)
                    }
                    (Operator::GreaterThanOrEqual, Value::Primitive(Primitive::Int(i2))) => {
                        i >= (*i2 as u128)
                    }
                    (Operator::Equal, Value::Primitive(Primitive::Int(i2))) => i == (*i2 as u128),
                    (Operator::NotEqual, Value::Primitive(Primitive::Int(i2))) => {
                        i != (*i2 as u128)
                    }
                    _ => false,
                },
                Value::Primitive(Primitive::Int(i)) => match (op, rhs) {
                    (Operator::LessThan, Value::Primitive(Primitive::Int(i2))) => i < *i2,
                    (Operator::GreaterThan, Value::Primitive(Primitive::Int(i2))) => i > *i2,
                    (Operator::LessThanOrEqual, Value::Primitive(Primitive::Int(i2))) => i <= *i2,
                    (Operator::GreaterThanOrEqual, Value::Primitive(Primitive::Int(i2))) => {
                        i >= *i2
                    }
                    (Operator::Equal, Value::Primitive(Primitive::Int(i2))) => i == *i2,
                    (Operator::NotEqual, Value::Primitive(Primitive::Int(i2))) => i != *i2,
                    _ => false,
                },
                Value::Primitive(Primitive::Float(i)) => match (op, rhs) {
                    (Operator::LessThan, Value::Primitive(Primitive::Float(i2))) => i < *i2,
                    (Operator::GreaterThan, Value::Primitive(Primitive::Float(i2))) => i > *i2,
                    (Operator::LessThanOrEqual, Value::Primitive(Primitive::Float(i2))) => i <= *i2,
                    (Operator::GreaterThanOrEqual, Value::Primitive(Primitive::Float(i2))) => {
                        i >= *i2
                    }
                    (Operator::Equal, Value::Primitive(Primitive::Float(i2))) => i == *i2,
                    (Operator::NotEqual, Value::Primitive(Primitive::Float(i2))) => i != *i2,
                    (Operator::LessThan, Value::Primitive(Primitive::Int(i2))) => {
                        (i.into_inner()) < *i2 as f64
                    }
                    (Operator::GreaterThan, Value::Primitive(Primitive::Int(i2))) => {
                        i.into_inner() > *i2 as f64
                    }
                    (Operator::LessThanOrEqual, Value::Primitive(Primitive::Int(i2))) => {
                        i.into_inner() <= *i2 as f64
                    }
                    (Operator::GreaterThanOrEqual, Value::Primitive(Primitive::Int(i2))) => {
                        i.into_inner() >= *i2 as f64
                    }
                    (Operator::Equal, Value::Primitive(Primitive::Int(i2))) => {
                        i.into_inner() == *i2 as f64
                    }
                    (Operator::NotEqual, Value::Primitive(Primitive::Int(i2))) => {
                        i.into_inner() != *i2 as f64
                    }

                    _ => false,
                },
                Value::Primitive(Primitive::String(s)) => match (op, rhs) {
                    (Operator::Equal, Value::Primitive(Primitive::String(s2))) => s == *s2,
                    (Operator::NotEqual, Value::Primitive(Primitive::String(s2))) => s != *s2,
                    _ => false,
                },
                _ => false,
            }
        }
    }
}
