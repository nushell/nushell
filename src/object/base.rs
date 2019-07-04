use crate::errors::ShellError;
use crate::evaluate::{evaluate_baseline_expr, Scope};
use crate::parser::{hir, Operator, Span, Spanned};
use crate::prelude::*;
use crate::Text;
use ansi_term::Color;
use chrono::{DateTime, Utc};
use chrono_humanize::Humanize;
use derive_new::new;
use ordered_float::OrderedFloat;
use serde::{ser::SerializeSeq, Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::time::SystemTime;

#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq, new, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Deserialize, Serialize)]
pub enum Primitive {
    Nothing,
    Int(i64),
    #[allow(unused)]
    Float(OF64),
    Bytes(u64),
    String(String),
    Boolean(bool),
    Date(DateTime<Utc>),

    EndOfStream,
}

impl Primitive {
    crate fn type_name(&self) -> String {
        use Primitive::*;

        match self {
            Nothing => "nothing",
            EndOfStream => "end-of-stream",
            Int(_) => "int",
            Float(_) => "float",
            Bytes(_) => "bytes",
            String(_) => "string",
            Boolean(_) => "boolean",
            Date(_) => "date",
        }
        .to_string()
    }

    crate fn debug(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Primitive::*;

        match self {
            Nothing => write!(f, "Nothing"),
            EndOfStream => write!(f, "EndOfStream"),
            Int(int) => write!(f, "{}", int),
            Float(float) => write!(f, "{:?}", float),
            Bytes(bytes) => write!(f, "{}", bytes),
            String(string) => write!(f, "{:?}", string),
            Boolean(boolean) => write!(f, "{}", boolean),
            Date(date) => write!(f, "{}", date),
        }
    }

    pub fn format(&self, field_name: Option<&String>) -> String {
        match self {
            Primitive::Nothing => format!("{}", Color::Black.bold().paint("-")),
            Primitive::EndOfStream => format!("{}", Color::Black.bold().paint("-")),
            Primitive::Bytes(b) => {
                let byte = byte_unit::Byte::from_bytes(*b as u128);

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
                (true, Some(s)) if !s.is_empty() => format!("{}", s),
                (false, Some(s)) if !s.is_empty() => format!(""),
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
    crate expressions: Vec<hir::Expression>,
    crate source: Text,
    crate span: Span,
}

impl Serialize for Block {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(None)?;

        let list = self
            .expressions
            .iter()
            .map(|e| e.source(&self.source.clone()));

        for item in list {
            seq.serialize_element(item.as_ref())?;
        }

        seq.end()
    }
}

impl Deserialize<'de> for Block {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        unimplemented!("deserialize block")
        // let s = "\"unimplemented deserialize block\"";
        // Ok(Block::new(
        //     TokenTreeBuilder::spanned_string((1, s.len() - 1), (0, s.len())),
        //     Text::from(s),
        // ))
    }
}

impl Block {
    pub fn invoke(&self, value: &Value) -> Result<Spanned<Value>, ShellError> {
        let scope = Scope::new(value.copy());

        if self.expressions.len() == 0 {
            return Ok(Spanned::from_item(Value::nothing(), self.span));
        }

        let mut last = None;

        for expr in self.expressions.iter() {
            last = Some(evaluate_baseline_expr(&expr, &(), &scope, &self.source)?)
        }

        Ok(last.unwrap())
    }
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Clone, Serialize, Deserialize)]
pub enum Value {
    Primitive(Primitive),
    Object(crate::object::Dictionary),
    List(Vec<Value>),
    Binary(Vec<u8>),

    #[allow(unused)]
    Block(Block),
    Filesystem,

    #[allow(unused)]
    Error(Box<ShellError>),
}

pub fn debug_list(values: &'a Vec<Value>) -> ValuesDebug<'a> {
    ValuesDebug { values }
}

pub struct ValuesDebug<'a> {
    values: &'a Vec<Value>,
}

impl fmt::Debug for ValuesDebug<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list()
            .entries(self.values.iter().map(|i| i.debug()))
            .finish()
    }
}

pub struct ValueDebug<'a> {
    value: &'a Value,
}

impl fmt::Debug for ValueDebug<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.value {
            Value::Primitive(p) => p.debug(f),
            Value::Object(o) => o.debug(f),
            Value::List(l) => debug_list(l).fmt(f),
            Value::Block(_) => write!(f, "[[block]]"),
            Value::Error(err) => write!(f, "[[error :: {} ]]", err),
            Value::Filesystem => write!(f, "[[filesystem]]"),
            Value::Binary(_) => write!(f, "[[binary]]"),
        }
    }
}

impl Spanned<Value> {
    crate fn spanned_type_name(&self) -> Spanned<String> {
        let name = self.type_name();
        Spanned::from_item(name, self.span)
    }
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
            Value::Binary(_) => format!("binary"),
        }
    }

    crate fn debug(&'a self) -> ValueDebug<'a> {
        ValueDebug { value: self }
    }

    pub fn data_descriptors(&self) -> Vec<String> {
        match self {
            Value::Primitive(_) => vec![],
            Value::Object(o) => o
                .entries
                .keys()
                .into_iter()
                .map(|x| x.to_string())
                .collect(),
            Value::Block(_) => vec![],
            Value::List(_) => vec![],
            Value::Error(_) => vec![],
            Value::Filesystem => vec![],
            Value::Binary(_) => vec![],
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

    pub fn get_data(&'a self, desc: &String) -> MaybeOwned<'a, Value> {
        match self {
            p @ Value::Primitive(_) => MaybeOwned::Borrowed(p),
            p @ Value::Filesystem => MaybeOwned::Borrowed(p),
            Value::Object(o) => o.get_data(desc),
            Value::Block(_) => MaybeOwned::Owned(Value::nothing()),
            Value::List(_) => MaybeOwned::Owned(Value::nothing()),
            Value::Error(e) => MaybeOwned::Owned(Value::string(&format!("{:#?}", e))),
            Value::Binary(_) => MaybeOwned::Owned(Value::nothing()),
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
            Value::Binary(b) => Value::Binary(b.clone()),
        }
    }

    crate fn format_leaf(&self, desc: Option<&String>) -> String {
        match self {
            Value::Primitive(p) => p.format(desc),
            Value::Block(b) => itertools::join(
                b.expressions
                    .iter()
                    .map(|e| e.source(&b.source).to_string()),
                "; ",
            ),
            Value::Object(_) => format!("[object Object]"),
            Value::List(_) => format!("[list List]"),
            Value::Error(e) => format!("{}", e),
            Value::Filesystem => format!("<filesystem>"),
            Value::Binary(_) => format!("<binary>"),
        }
    }

    #[allow(unused)]
    crate fn compare(&self, operator: &Operator, other: &Value) -> Result<bool, (String, String)> {
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

                Ok(result)
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
            Value::Primitive(Primitive::String(s)) => Ok(s.clone()),
            Value::Primitive(Primitive::Boolean(x)) => Ok(format!("{}", x)),
            Value::Primitive(Primitive::Float(x)) => Ok(format!("{}", x.into_inner())),
            Value::Primitive(Primitive::Int(x)) => Ok(format!("{}", x)),
            Value::Primitive(Primitive::Bytes(x)) => Ok(format!("{}", x)),
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
            Value::Primitive(Primitive::Bytes(b)) => Ok(*b as i64),
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

    crate fn is_true(&self) -> bool {
        match self {
            Value::Primitive(Primitive::Boolean(true)) => true,
            _ => false,
        }
    }

    pub fn string(s: impl Into<String>) -> Value {
        Value::Primitive(Primitive::String(s.into()))
    }

    pub fn bytes(s: impl Into<u64>) -> Value {
        Value::Primitive(Primitive::Bytes(s.into()))
    }

    pub fn int(s: impl Into<i64>) -> Value {
        Value::Primitive(Primitive::Int(s.into()))
    }

    pub fn float(s: impl Into<OF64>) -> Value {
        Value::Primitive(Primitive::Float(s.into()))
    }

    pub fn boolean(s: impl Into<bool>) -> Value {
        Value::Primitive(Primitive::Boolean(s.into()))
    }

    pub fn system_date(s: SystemTime) -> Value {
        Value::Primitive(Primitive::Date(s.into()))
    }

    #[allow(unused)]
    pub fn date_from_str(s: &str) -> Result<Value, ShellError> {
        let date = DateTime::parse_from_rfc3339(s)
            .map_err(|err| ShellError::string(&format!("Date parse error: {}", err)))?;

        let date = date.with_timezone(&chrono::offset::Utc);

        Ok(Value::Primitive(Primitive::Date(date)))
    }

    #[allow(unused)]
    pub fn system_date_result(s: Result<SystemTime, std::io::Error>) -> Value {
        match s {
            Ok(time) => Value::Primitive(Primitive::Date(time.into())),
            Err(err) => Value::Error(Box::new(ShellError::string(format!("{}", err)))),
        }
    }

    pub fn nothing() -> Value {
        Value::Primitive(Primitive::Nothing)
    }

    #[allow(unused)]
    pub fn list(values: impl Into<Vec<Value>>) -> Value {
        Value::List(values.into())
    }
}

crate fn select_fields(obj: &Value, fields: &[String]) -> crate::object::Dictionary {
    let mut out = crate::object::Dictionary::default();

    let descs = obj.data_descriptors();

    for field in fields {
        match descs.iter().find(|d| *d == field) {
            None => out.add(field, Value::nothing()),
            Some(desc) => out.add(desc.clone(), obj.get_data(desc).borrow().copy()),
        }
    }

    out
}

crate fn reject_fields(obj: &Value, fields: &[String]) -> crate::object::Dictionary {
    let mut out = crate::object::Dictionary::default();

    let descs = obj.data_descriptors();

    for desc in descs {
        match desc {
            x if fields.iter().any(|field| *field == x) => continue,
            _ => out.add(desc.clone(), obj.get_data(&desc).borrow().copy()),
        }
    }

    out
}

#[allow(unused)]
crate fn find(obj: &Value, field: &str, op: &Operator, rhs: &Value) -> bool {
    let descs = obj.data_descriptors();
    match descs.iter().find(|d| *d == field) {
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
                    (Operator::LessThan, Value::Primitive(Primitive::Int(i2))) => i < (*i2 as u64),
                    (Operator::GreaterThan, Value::Primitive(Primitive::Int(i2))) => {
                        i > (*i2 as u64)
                    }
                    (Operator::LessThanOrEqual, Value::Primitive(Primitive::Int(i2))) => {
                        i <= (*i2 as u64)
                    }
                    (Operator::GreaterThanOrEqual, Value::Primitive(Primitive::Int(i2))) => {
                        i >= (*i2 as u64)
                    }
                    (Operator::Equal, Value::Primitive(Primitive::Int(i2))) => i == (*i2 as u64),
                    (Operator::NotEqual, Value::Primitive(Primitive::Int(i2))) => i != (*i2 as u64),
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

fn coerce_compare(left: &Value, right: &Value) -> Result<CompareValues, (String, String)> {
    match (left, right) {
        (Value::Primitive(left), Value::Primitive(right)) => coerce_compare_primitive(left, right),

        _ => Err((left.type_name(), right.type_name())),
    }
}

fn coerce_compare_primitive(
    left: &Primitive,
    right: &Primitive,
) -> Result<CompareValues, (String, String)> {
    use Primitive::*;

    Ok(match (left, right) {
        (Int(left), Int(right)) => CompareValues::Ints(*left, *right),
        (Float(left), Int(right)) => CompareValues::Floats(*left, (*right as f64).into()),
        (Int(left), Float(right)) => CompareValues::Floats((*left as f64).into(), *right),
        (Int(left), Bytes(right)) => CompareValues::Bytes(*left as i128, *right as i128),
        (Bytes(left), Int(right)) => CompareValues::Bytes(*left as i128, *right as i128),
        (String(left), String(right)) => CompareValues::String(left.clone(), right.clone()),
        _ => return Err((left.type_name(), right.type_name())),
    })
}
