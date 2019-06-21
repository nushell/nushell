use crate::errors::ShellError;
use crate::evaluate::{evaluate_expr, Scope};
use crate::object::DataDescriptor;
use crate::parser::ast::{self, Operator};
use crate::parser::lexer::Spanned;
use crate::prelude::*;
use ansi_term::Color;
use chrono::{DateTime, Utc};
use chrono_humanize::Humanize;
use derive_new::new;
use ordered_float::OrderedFloat;
use std::time::SystemTime;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq, new)]
pub struct OF64 {
    crate inner: OrderedFloat<f64>,
}

impl OF64 {
    crate fn into_inner(&self) -> f64 {
        self.inner.into_inner()
    }
}

impl From<f64> for OF64 {
    fn from(float: f64) -> Self {
        OF64::new(OrderedFloat(float))
    }
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Deserialize)]
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

impl Serialize for Primitive {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Primitive::Nothing => serializer.serialize_i32(0),
            Primitive::Int(i) => serializer.serialize_i64(*i),
            Primitive::Float(OF64 { inner: f }) => serializer.serialize_f64(f.into_inner()),
            Primitive::Bytes(b) => serializer.serialize_u128(*b),
            Primitive::String(ref s) => serializer.serialize_str(s),
            Primitive::Boolean(b) => serializer.serialize_bool(*b),
            Primitive::Date(d) => serializer.serialize_str(&d.to_string()),
        }
    }
}

impl Primitive {
    crate fn type_name(&self) -> String {
        use Primitive::*;

        match self {
            Nothing => "nothing",
            Int(_) => "int",
            Float(_) => "float",
            Bytes(_) => "bytes",
            String(_) => "string",
            Boolean(_) => "boolean",
            Date(_) => "date",
        }
        .to_string()
    }

    crate fn format(&self, field_name: Option<&DataDescriptor>) -> String {
        match self {
            Primitive::Nothing => format!("{}", Color::Black.bold().paint("-")),
            Primitive::Bytes(b) => {
                let byte = byte_unit::Byte::from_bytes(*b);

                if byte.get_bytes() == 0u128 {
                    return Color::Black.bold().paint("Empty".to_string()).to_string();
                }

                let byte = byte.get_appropriate_unit(false);

                match byte.get_unit() {
                    byte_unit::ByteUnit::B => format!("{}", byte.format(0)),
                    _ => format!("{}", byte.format(1)),
                }
            }
            Primitive::Int(i) => format!("{}", i),
            Primitive::Float(OF64 { inner: f }) => format!("{:.*}", 2, f.into_inner()),
            Primitive::String(s) => format!("{}", s),
            Primitive::Boolean(b) => match (b, field_name) {
                (true, None) => format!("Yes"),
                (false, None) => format!("No"),
                (true, Some(s)) if s.is_string_name() => format!("{}", s.display_header()),
                (false, Some(s)) if s.is_string_name() => format!(""),
                (true, Some(_)) => format!("Yes"),
                (false, Some(_)) => format!("No"),
            },
            Primitive::Date(d) => format!("{}", d.humanize()),
        }
    }
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone, new, Serialize)]
pub struct Operation {
    crate left: Value,
    crate operator: Operator,
    crate right: Value,
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone, new)]
pub struct Block {
    crate expression: ast::Expression,
}

impl Serialize for Block {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.expression.print())
    }
}

impl Deserialize<'de> for Block {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut builder = ast::ExpressionBuilder::new();
        let expr: ast::Expression = builder.string("Unserializable block");

        Ok(Block::new(expr))
    }
}

impl Block {
    pub fn invoke(&self, value: &Value) -> Result<Spanned<Value>, ShellError> {
        let scope = Scope::new(value.copy());
        evaluate_expr(&self.expression, &scope)
    }
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone)]
pub enum Value {
    Primitive(Primitive),
    Object(crate::object::Dictionary),
    List(Vec<Value>),
    Block(Block),
    Filesystem,

    #[allow(unused)]
    Error(Box<ShellError>),
}

impl Value {
    crate fn type_name(&self) -> String {
        match self {
            Value::Primitive(p) => p.type_name(),
            Value::Object(_) => format!("object"),
            Value::List(_) => format!("list"),
            Value::Block(_) => format!("block"),
            Value::Error(_) => format!("error"),
            Value::Filesystem => format!("filesystem"),
        }
    }

    crate fn data_descriptors(&self) -> Vec<DataDescriptor> {
        match self {
            Value::Primitive(_) => vec![DataDescriptor::value_of()],
            Value::Object(o) => o.data_descriptors(),
            Value::Block(_) => vec![DataDescriptor::value_of()],
            Value::List(_) => vec![],
            Value::Error(_) => vec![DataDescriptor::value_of()],
            Value::Filesystem => vec![],
        }
    }

    crate fn get_data_by_key(&'a self, name: &str) -> Option<&Value> {
        match self {
            Value::Object(o) => o.get_data_by_key(name),
            Value::List(l) => {
                for item in l {
                    match item {
                        Value::Object(o) => match o.get_data_by_key(name) {
                            Some(v) => return Some(v),
                            None => {}
                        },
                        _ => {}
                    }
                }
                None
            }
            _ => None,
        }
    }

    crate fn get_data_by_index(&'a self, idx: usize) -> Option<&Value> {
        match self {
            Value::List(l) => l.iter().nth(idx),
            _ => None,
        }
    }

    crate fn get_data(&'a self, desc: &DataDescriptor) -> MaybeOwned<'a, Value> {
        match self {
            p @ Value::Primitive(_) => MaybeOwned::Borrowed(p),
            p @ Value::Filesystem => MaybeOwned::Borrowed(p),
            Value::Object(o) => o.get_data(desc),
            Value::Block(_) => MaybeOwned::Owned(Value::nothing()),
            Value::List(_) => MaybeOwned::Owned(Value::nothing()),
            Value::Error(e) => MaybeOwned::Owned(Value::string(&format!("{:#?}", e))),
        }
    }

    crate fn copy(&self) -> Value {
        match self {
            Value::Primitive(p) => Value::Primitive(p.clone()),
            Value::Object(o) => Value::Object(o.copy_dict()),
            Value::Block(b) => Value::Block(b.clone()),
            Value::List(l) => {
                let list = l.iter().map(|i| i.copy()).collect();
                Value::List(list)
            }
            Value::Error(e) => Value::Error(Box::new(e.copy_error())),
            Value::Filesystem => Value::Filesystem,
        }
    }

    crate fn format_leaf(&self, desc: Option<&DataDescriptor>) -> String {
        match self {
            Value::Primitive(p) => p.format(desc),
            Value::Block(b) => b.expression.print(),
            Value::Object(_) => format!("[object Object]"),
            Value::List(_) => format!("[list List]"),
            Value::Error(e) => format!("{}", e),
            Value::Filesystem => format!("<filesystem>"),
        }
    }

    crate fn compare(&self, operator: &ast::Operator, other: &Value) -> Option<bool> {
        match operator {
            _ => {
                let coerced = coerce_compare(self, other)?;
                let ordering = coerced.compare();

                use std::cmp::Ordering;

                let result = match (operator, ordering) {
                    (Operator::Equal, Ordering::Equal) => true,
                    (Operator::NotEqual, Ordering::Less)
                    | (Operator::NotEqual, Ordering::Greater) => true,
                    (Operator::LessThan, Ordering::Less) => true,
                    (Operator::GreaterThan, Ordering::Greater) => true,
                    (Operator::GreaterThanOrEqual, Ordering::Greater)
                    | (Operator::GreaterThanOrEqual, Ordering::Equal) => true,
                    (Operator::LessThanOrEqual, Ordering::Less)
                    | (Operator::LessThanOrEqual, Ordering::Equal) => true,
                    _ => false,
                };

                Some(result)
            }
        }
    }

    #[allow(unused)]
    crate fn is_string(&self, expected: &str) -> bool {
        match self {
            Value::Primitive(Primitive::String(s)) if s == expected => true,
            other => false,
        }
    }

    crate fn as_pair(&self) -> Result<(Value, Value), ShellError> {
        match self {
            Value::List(list) if list.len() == 2 => Ok((list[0].clone(), list[1].clone())),
            other => Err(ShellError::string(format!(
                "Expected pair, got {:?}",
                other
            ))),
        }
    }

    crate fn as_string(&self) -> Result<String, ShellError> {
        match self {
            Value::Primitive(Primitive::String(x)) => Ok(format!("{}", x)),
            Value::Primitive(Primitive::Boolean(x)) => Ok(format!("{}", x)),
            Value::Primitive(Primitive::Float(x)) => Ok(format!("{}", x.into_inner())),
            Value::Primitive(Primitive::Int(x)) => Ok(format!("{}", x)),
            Value::Primitive(Primitive::Bytes(x)) => Ok(format!("{}", x)),
            //Value::Primitive(Primitive::String(s)) => Ok(s.clone()),
            // TODO: this should definitely be more general with better errors
            other => Err(ShellError::string(format!(
                "Expected string, got {:?}",
                other
            ))),
        }
    }

    crate fn as_i64(&self) -> Result<i64, ShellError> {
        match self {
            Value::Primitive(Primitive::Int(i)) => Ok(*i),
            Value::Primitive(Primitive::Bytes(b)) if *b <= std::i64::MAX as u128 => Ok(*b as i64),
            // TODO: this should definitely be more general with better errors
            other => Err(ShellError::string(format!(
                "Expected integer, got {:?}",
                other
            ))),
        }
    }

    crate fn as_block(&self) -> Result<Block, ShellError> {
        match self {
            Value::Block(block) => Ok(block.clone()),
            // TODO: this should definitely be more general with better errors
            other => Err(ShellError::string(format!(
                "Expected block, got {:?}",
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

    crate fn is_true(&self) -> bool {
        match self {
            Value::Primitive(Primitive::Boolean(true)) => true,
            _ => false,
        }
    }

    crate fn block(e: ast::Expression) -> Value {
        Value::Block(Block::new(e))
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

    crate fn boolean(s: impl Into<bool>) -> Value {
        Value::Primitive(Primitive::Boolean(s.into()))
    }

    crate fn system_date(s: SystemTime) -> Value {
        Value::Primitive(Primitive::Date(s.into()))
    }

    #[allow(unused)]
    crate fn date_from_str(s: &str) -> Result<Value, ShellError> {
        let date = DateTime::parse_from_rfc3339(s)
            .map_err(|err| ShellError::string(&format!("Date parse error: {}", err)))?;

        let date = date.with_timezone(&chrono::offset::Utc);

        Ok(Value::Primitive(Primitive::Date(date)))
    }

    #[allow(unused)]
    crate fn system_date_result(s: Result<SystemTime, std::io::Error>) -> Value {
        match s {
            Ok(time) => Value::Primitive(Primitive::Date(time.into())),
            Err(err) => Value::Error(Box::new(ShellError::string(format!("{}", err)))),
        }
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

#[allow(unused)]
crate fn find(obj: &Value, field: &str, op: &Operator, rhs: &Value) -> bool {
    let descs = obj.data_descriptors();
    match descs.iter().find(|d| d.name.is_string(field)) {
        None => false,
        Some(desc) => {
            let v = obj.get_data(desc).borrow().copy();

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

enum CompareValues {
    Ints(i64, i64),
    Floats(OF64, OF64),
    Bytes(i128, i128),
    String(String, String),
}

impl CompareValues {
    fn compare(&self) -> std::cmp::Ordering {
        match self {
            CompareValues::Ints(left, right) => left.cmp(right),
            CompareValues::Floats(left, right) => left.cmp(right),
            CompareValues::Bytes(left, right) => left.cmp(right),
            CompareValues::String(left, right) => left.cmp(right),
        }
    }
}

fn coerce_compare(left: &Value, right: &Value) -> Option<CompareValues> {
    match (left, right) {
        (Value::Primitive(left), Value::Primitive(right)) => coerce_compare_primitive(left, right),

        _ => None,
    }
}

fn coerce_compare_primitive(left: &Primitive, right: &Primitive) -> Option<CompareValues> {
    use Primitive::*;

    match (left, right) {
        (Int(left), Int(right)) => Some(CompareValues::Ints(*left, *right)),
        (Float(left), Int(right)) => Some(CompareValues::Floats(*left, (*right as f64).into())),
        (Int(left), Float(right)) => Some(CompareValues::Floats((*left as f64).into(), *right)),
        (Int(left), Bytes(right)) => Some(CompareValues::Bytes(*left as i128, *right as i128)),
        (Bytes(left), Int(right)) => Some(CompareValues::Bytes(*left as i128, *right as i128)),
        (String(left), String(right)) => Some(CompareValues::String(left.clone(), right.clone())),
        _ => None,
    }
}
