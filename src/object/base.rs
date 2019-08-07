use crate::errors::ShellError;
use crate::evaluate::{evaluate_baseline_expr, Scope};
use crate::object::TaggedDictBuilder;
use crate::parser::{hir, Operator};
use crate::prelude::*;
use crate::Text;
use ansi_term::Color;
use chrono::{DateTime, Utc};
use chrono_humanize::Humanize;
use derive_new::new;
use ordered_float::OrderedFloat;
use serde::{ser::SerializeSeq, Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::path::PathBuf;
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
    Path(PathBuf),

    // Stream markers (used as bookend markers rather than actual values)
    BeginningOfStream,
    EndOfStream,
}

impl Primitive {
    crate fn type_name(&self) -> String {
        use Primitive::*;

        match self {
            Nothing => "nothing",
            BeginningOfStream => "beginning-of-stream",
            EndOfStream => "end-of-stream",
            Path(_) => "path",
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
            BeginningOfStream => write!(f, "BeginningOfStream"),
            EndOfStream => write!(f, "EndOfStream"),
            Int(int) => write!(f, "{}", int),
            Path(path) => write!(f, "{}", path.display()),
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
            Primitive::BeginningOfStream => format!("{}", Color::Black.bold().paint("-")),
            Primitive::EndOfStream => format!("{}", Color::Black.bold().paint("-")),
            Primitive::Path(p) => format!("{}", p.display()),
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
    }
}

impl Block {
    pub fn invoke(&self, value: &Tagged<Value>) -> Result<Tagged<Value>, ShellError> {
        let scope = Scope::new(value.clone());

        if self.expressions.len() == 0 {
            return Ok(Value::nothing().simple_spanned(self.span));
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
    #[serde(with = "serde_bytes")]
    Binary(Vec<u8>),
    List(Vec<Tagged<Value>>),
    #[allow(unused)]
    Block(Block),
}

pub fn debug_list(values: &'a Vec<Tagged<Value>>) -> ValuesDebug<'a> {
    ValuesDebug { values }
}

pub struct ValuesDebug<'a> {
    values: &'a Vec<Tagged<Value>>,
}

impl fmt::Debug for ValuesDebug<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list()
            .entries(self.values.iter().map(|i| i.debug()))
            .finish()
    }
}

pub struct ValueDebug<'a> {
    value: &'a Tagged<Value>,
}

impl fmt::Debug for ValueDebug<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.value.item() {
            Value::Primitive(p) => p.debug(f),
            Value::Object(o) => o.debug(f),
            Value::List(l) => debug_list(l).fmt(f),
            Value::Block(_) => write!(f, "[[block]]"),
            Value::Binary(_) => write!(f, "[[binary]]"),
        }
    }
}

impl Tagged<Value> {
    crate fn tagged_type_name(&self) -> Tagged<String> {
        let name = self.type_name();
        Tagged::from_simple_spanned_item(name, self.span())
    }
}

impl std::convert::TryFrom<&'a Tagged<Value>> for Block {
    type Error = ShellError;

    fn try_from(value: &'a Tagged<Value>) -> Result<Block, ShellError> {
        match value.item() {
            Value::Block(block) => Ok(block.clone()),
            v => Err(ShellError::type_error(
                "Block",
                value.copy_span(v.type_name()),
            )),
        }
    }
}

impl std::convert::TryFrom<&'a Tagged<Value>> for i64 {
    type Error = ShellError;

    fn try_from(value: &'a Tagged<Value>) -> Result<i64, ShellError> {
        match value.item() {
            Value::Primitive(Primitive::Int(int)) => Ok(*int),
            v => Err(ShellError::type_error(
                "Integer",
                value.copy_span(v.type_name()),
            )),
        }
    }
}

pub enum Switch {
    Present,
    Absent,
}

impl Switch {
    pub fn is_present(&self) -> bool {
        match self {
            Switch::Present => true,
            Switch::Absent => false,
        }
    }
}

impl std::convert::TryFrom<Option<&'a Tagged<Value>>> for Switch {
    type Error = ShellError;

    fn try_from(value: Option<&'a Tagged<Value>>) -> Result<Switch, ShellError> {
        match value {
            None => Ok(Switch::Absent),
            Some(value) => match value.item() {
                Value::Primitive(Primitive::Boolean(true)) => Ok(Switch::Present),
                v => Err(ShellError::type_error(
                    "Boolean",
                    value.copy_span(v.type_name()),
                )),
            },
        }
    }
}

impl Tagged<Value> {
    crate fn debug(&'a self) -> ValueDebug<'a> {
        ValueDebug { value: self }
    }
}

impl Value {
    crate fn type_name(&self) -> String {
        match self {
            Value::Primitive(p) => p.type_name(),
            Value::Object(_) => format!("object"),
            Value::List(_) => format!("list"),
            Value::Block(_) => format!("block"),
            Value::Binary(_) => format!("binary"),
        }
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
            Value::Binary(_) => vec![],
        }
    }

    crate fn get_data_by_key(&'a self, name: &str) -> Option<&Tagged<Value>> {
        match self {
            Value::Object(o) => o.get_data_by_key(name),
            Value::List(l) => {
                for item in l {
                    match item {
                        Tagged {
                            item: Value::Object(o),
                            ..
                        } => match o.get_data_by_key(name) {
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

    #[allow(unused)]
    crate fn get_data_by_index(&'a self, idx: usize) -> Option<&Tagged<Value>> {
        match self {
            Value::List(l) => l.iter().nth(idx),
            _ => None,
        }
    }

    pub fn get_data_by_path(&'a self, tag: Tag, path: &str) -> Option<Tagged<&Value>> {
        let mut current = self;
        for p in path.split(".") {
            match current.get_data_by_key(p) {
                Some(v) => current = v,
                None => return None,
            }
        }

        Some(Tagged::from_item(current, tag))
    }

    pub fn insert_data_at_path(
        &'a self,
        tag: Tag,
        path: &str,
        new_value: Value,
    ) -> Option<Tagged<Value>> {
        let mut new_obj = self.clone();

        let split_path: Vec<_> = path.split(".").collect();

        if let Value::Object(ref mut o) = new_obj {
            let mut current = o;
            for idx in 0..split_path.len() - 1 {
                match current.entries.get_mut(split_path[idx]) {
                    Some(next) => {
                        if idx == (split_path.len() - 2) {
                            match &mut next.item {
                                Value::Object(o) => {
                                    o.entries.insert(
                                        split_path[idx + 1].to_string(),
                                        Tagged::from_item(new_value, tag),
                                    );
                                }
                                _ => {}
                            }

                            return Some(Tagged::from_item(new_obj, tag));
                        } else {
                            match next.item {
                                Value::Object(ref mut o) => {
                                    current = o;
                                }
                                _ => return None,
                            }
                        }
                    }
                    _ => return None,
                }
            }
        }

        None
    }

    pub fn replace_data_at_path(
        &'a self,
        tag: Tag,
        path: &str,
        replaced_value: Value,
    ) -> Option<Tagged<Value>> {
        let mut new_obj = self.clone();

        let split_path: Vec<_> = path.split(".").collect();

        if let Value::Object(ref mut o) = new_obj {
            let mut current = o;
            for idx in 0..split_path.len() {
                match current.entries.get_mut(split_path[idx]) {
                    Some(next) => {
                        if idx == (split_path.len() - 1) {
                            *next = Tagged::from_item(replaced_value, tag);
                            return Some(Tagged::from_item(new_obj, tag));
                        } else {
                            match next.item {
                                Value::Object(ref mut o) => {
                                    current = o;
                                }
                                _ => return None,
                            }
                        }
                    }
                    _ => return None,
                }
            }
        }

        None
    }

    pub fn get_data(&'a self, desc: &String) -> MaybeOwned<'a, Value> {
        match self {
            p @ Value::Primitive(_) => MaybeOwned::Borrowed(p),
            Value::Object(o) => o.get_data(desc),
            Value::Block(_) => MaybeOwned::Owned(Value::nothing()),
            Value::List(_) => MaybeOwned::Owned(Value::nothing()),
            Value::Binary(_) => MaybeOwned::Owned(Value::nothing()),
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
            Value::Object(_) => format!("[{}]", self.type_name()),
            Value::List(l) => format!(
                "[{} {}]",
                l.len(),
                if l.len() == 1 { "item" } else { "items" }
            ),
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

    crate fn as_pair(&self) -> Result<(Tagged<Value>, Tagged<Value>), ShellError> {
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

    pub fn nothing() -> Value {
        Value::Primitive(Primitive::Nothing)
    }
}

crate fn select_fields(obj: &Value, fields: &[String], tag: impl Into<Tag>) -> Tagged<Value> {
    let mut out = TaggedDictBuilder::new(tag);

    let descs = obj.data_descriptors();

    for field in fields {
        match descs.iter().find(|d| *d == field) {
            None => out.insert(field, Value::nothing()),
            Some(desc) => out.insert(desc.clone(), obj.get_data(desc).borrow().clone()),
        }
    }

    out.into_tagged_value()
}

crate fn reject_fields(obj: &Value, fields: &[String], tag: impl Into<Tag>) -> Tagged<Value> {
    let mut out = TaggedDictBuilder::new(tag);

    let descs = obj.data_descriptors();

    for desc in descs {
        match desc {
            x if fields.iter().any(|field| *field == x) => continue,
            _ => out.insert(desc.clone(), obj.get_data(&desc).borrow().clone()),
        }
    }

    out.into_tagged_value()
}

#[allow(unused)]
crate fn find(obj: &Value, field: &str, op: &Operator, rhs: &Value) -> bool {
    let descs = obj.data_descriptors();
    match descs.iter().find(|d| *d == field) {
        None => false,
        Some(desc) => {
            let v = obj.get_data(desc).borrow().clone();

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
