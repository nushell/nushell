use crate::context::CommandRegistry;
use crate::data::TaggedDictBuilder;
use crate::errors::ShellError;
use crate::evaluate::{evaluate_baseline_expr, Scope};
use crate::parser::{hir, Operator};
use crate::prelude::*;
use crate::Text;
use chrono::{DateTime, Utc};
use chrono_humanize::Humanize;
use derive_new::new;
use log::trace;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;
use std::time::SystemTime;

mod serde_bigint {
    use num_traits::cast::FromPrimitive;
    use num_traits::cast::ToPrimitive;

    pub fn serialize<S>(big_int: &super::BigInt, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serde::Serialize::serialize(
            &big_int
                .to_i64()
                .ok_or(serde::ser::Error::custom("expected a i64-sized bignum"))?,
            serializer,
        )
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<super::BigInt, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let x: i64 = serde::Deserialize::deserialize(deserializer)?;
        Ok(super::BigInt::from_i64(x)
            .ok_or(serde::de::Error::custom("expected a i64-sized bignum"))?)
    }
}

mod serde_bigdecimal {
    use num_traits::cast::FromPrimitive;
    use num_traits::cast::ToPrimitive;

    pub fn serialize<S>(big_decimal: &super::BigDecimal, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serde::Serialize::serialize(
            &big_decimal
                .to_f64()
                .ok_or(serde::ser::Error::custom("expected a f64-sized bignum"))?,
            serializer,
        )
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<super::BigDecimal, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let x: f64 = serde::Deserialize::deserialize(deserializer)?;
        Ok(super::BigDecimal::from_f64(x)
            .ok_or(serde::de::Error::custom("expected a f64-sized bigdecimal"))?)
    }
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Deserialize, Serialize)]
pub enum Primitive {
    Nothing,
    #[serde(with = "serde_bigint")]
    Int(BigInt),
    #[serde(with = "serde_bigdecimal")]
    Decimal(BigDecimal),
    Bytes(u64),
    String(String),
    Pattern(String),
    Boolean(bool),
    Date(DateTime<Utc>),
    Path(PathBuf),
    #[serde(with = "serde_bytes")]
    Binary(Vec<u8>),

    // Stream markers (used as bookend markers rather than actual values)
    BeginningOfStream,
    EndOfStream,
}

impl From<BigDecimal> for Primitive {
    fn from(decimal: BigDecimal) -> Primitive {
        Primitive::Decimal(decimal)
    }
}

impl From<f64> for Primitive {
    fn from(float: f64) -> Primitive {
        Primitive::Decimal(BigDecimal::from_f64(float).unwrap())
    }
}

impl Primitive {
    pub(crate) fn type_name(&self) -> String {
        use Primitive::*;

        match self {
            Nothing => "nothing",
            BeginningOfStream => "beginning-of-stream",
            EndOfStream => "end-of-stream",
            Path(_) => "path",
            Int(_) => "int",
            Decimal(_) => "decimal",
            Bytes(_) => "bytes",
            Pattern(_) => "pattern",
            String(_) => "string",
            Boolean(_) => "boolean",
            Date(_) => "date",
            Binary(_) => "binary",
        }
        .to_string()
    }

    pub(crate) fn debug(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Primitive::*;

        match self {
            Nothing => write!(f, "Nothing"),
            BeginningOfStream => write!(f, "BeginningOfStream"),
            EndOfStream => write!(f, "EndOfStream"),
            Int(int) => write!(f, "{}", int),
            Path(path) => write!(f, "{}", path.display()),
            Decimal(decimal) => write!(f, "{}", decimal),
            Bytes(bytes) => write!(f, "{}", bytes),
            Pattern(string) => write!(f, "{:?}", string),
            String(string) => write!(f, "{:?}", string),
            Boolean(boolean) => write!(f, "{}", boolean),
            Date(date) => write!(f, "{}", date),
            Binary(binary) => write!(f, "{:?}", binary),
        }
    }

    pub fn number(number: impl Into<Number>) -> Primitive {
        let number = number.into();

        match number {
            Number::Int(int) => Primitive::Int(int),
            Number::Decimal(decimal) => Primitive::Decimal(decimal),
        }
    }

    pub fn format(&self, field_name: Option<&String>) -> String {
        match self {
            Primitive::Nothing => String::new(),
            Primitive::BeginningOfStream => String::new(),
            Primitive::EndOfStream => String::new(),
            Primitive::Path(p) => format!("{}", p.display()),
            Primitive::Bytes(b) => {
                let byte = byte_unit::Byte::from_bytes(*b as u128);

                if byte.get_bytes() == 0u128 {
                    return "—".to_string();
                }

                let byte = byte.get_appropriate_unit(false);

                match byte.get_unit() {
                    byte_unit::ByteUnit::B => format!("{} B ", byte.get_value()),
                    _ => format!("{}", byte.format(1)),
                }
            }
            Primitive::Int(i) => format!("{}", i),
            Primitive::Decimal(decimal) => format!("{}", decimal),
            Primitive::Pattern(s) => format!("{}", s),
            Primitive::String(s) => format!("{}", s),
            Primitive::Boolean(b) => match (b, field_name) {
                (true, None) => format!("Yes"),
                (false, None) => format!("No"),
                (true, Some(s)) if !s.is_empty() => format!("{}", s),
                (false, Some(s)) if !s.is_empty() => format!(""),
                (true, Some(_)) => format!("Yes"),
                (false, Some(_)) => format!("No"),
            },
            Primitive::Binary(_) => format!("<binary>"),
            Primitive::Date(d) => format!("{}", d.humanize()),
        }
    }

    pub fn style(&self) -> &'static str {
        match self {
            Primitive::Bytes(0) => "c", // centre 'missing' indicator
            Primitive::Int(_) | Primitive::Bytes(_) | Primitive::Decimal(_) => "r",
            _ => "",
        }
    }
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone, new, Serialize)]
pub struct Operation {
    pub(crate) left: Value,
    pub(crate) operator: Operator,
    pub(crate) right: Value,
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone, Serialize, Deserialize, new)]
pub struct Block {
    pub(crate) expressions: Vec<hir::Expression>,
    pub(crate) source: Text,
    pub(crate) tag: Tag,
}

impl Block {
    pub fn invoke(&self, value: &Tagged<Value>) -> Result<Tagged<Value>, ShellError> {
        let scope = Scope::new(value.clone());

        if self.expressions.len() == 0 {
            return Ok(Value::nothing().tagged(self.tag));
        }

        let mut last = None;

        trace!(
            "EXPRS = {:?}",
            self.expressions
                .iter()
                .map(|e| format!("{}", e))
                .collect::<Vec<_>>()
        );

        for expr in self.expressions.iter() {
            last = Some(evaluate_baseline_expr(
                &expr,
                &CommandRegistry::empty(),
                &scope,
                &self.source,
            )?)
        }

        Ok(last.unwrap())
    }
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Clone, Serialize, Deserialize)]
pub enum Value {
    Primitive(Primitive),
    Row(crate::data::Dictionary),
    Table(Vec<Tagged<Value>>),

    Block(Block),
}

impl Into<Value> for Number {
    fn into(self) -> Value {
        match self {
            Number::Int(int) => Value::int(int),
            Number::Decimal(decimal) => Value::decimal(decimal),
        }
    }
}

impl Into<Value> for &Number {
    fn into(self) -> Value {
        match self {
            Number::Int(int) => Value::int(int.clone()),
            Number::Decimal(decimal) => Value::decimal(decimal.clone()),
        }
    }
}

pub fn debug_list(values: &Vec<Tagged<Value>>) -> ValuesDebug<'_> {
    ValuesDebug { values }
}

pub struct ValuesDebug<'a> {
    values: &'a Vec<Tagged<Value>>,
}

impl fmt::Debug for ValuesDebug<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list()
            .entries(self.values.iter().map(|i| i.debug()))
            .finish()
    }
}

pub struct ValueDebug<'a> {
    value: &'a Tagged<Value>,
}

impl fmt::Debug for ValueDebug<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.value.item() {
            Value::Primitive(p) => p.debug(f),
            Value::Row(o) => o.debug(f),
            Value::Table(l) => debug_list(l).fmt(f),
            Value::Block(_) => write!(f, "[[block]]"),
        }
    }
}

impl Tagged<Value> {
    pub(crate) fn tagged_type_name(&self) -> Tagged<String> {
        let name = self.type_name();
        Tagged::from_item(name, self.tag())
    }
}

impl std::convert::TryFrom<&Tagged<Value>> for Block {
    type Error = ShellError;

    fn try_from(value: &Tagged<Value>) -> Result<Block, ShellError> {
        match value.item() {
            Value::Block(block) => Ok(block.clone()),
            v => Err(ShellError::type_error(
                "Block",
                value.copy_tag(v.type_name()),
            )),
        }
    }
}

impl std::convert::TryFrom<&Tagged<Value>> for i64 {
    type Error = ShellError;

    fn try_from(value: &Tagged<Value>) -> Result<i64, ShellError> {
        match value.item() {
            Value::Primitive(Primitive::Int(int)) => {
                int.tagged(value.tag).coerce_into("converting to i64")
            }
            v => Err(ShellError::type_error(
                "Integer",
                value.copy_tag(v.type_name()),
            )),
        }
    }
}

impl std::convert::TryFrom<&Tagged<Value>> for String {
    type Error = ShellError;

    fn try_from(value: &Tagged<Value>) -> Result<String, ShellError> {
        match value.item() {
            Value::Primitive(Primitive::String(s)) => Ok(s.clone()),
            v => Err(ShellError::type_error(
                "String",
                value.copy_tag(v.type_name()),
            )),
        }
    }
}

impl std::convert::TryFrom<&Tagged<Value>> for Vec<u8> {
    type Error = ShellError;

    fn try_from(value: &Tagged<Value>) -> Result<Vec<u8>, ShellError> {
        match value.item() {
            Value::Primitive(Primitive::Binary(b)) => Ok(b.clone()),
            v => Err(ShellError::type_error(
                "Binary",
                value.copy_tag(v.type_name()),
            )),
        }
    }
}

impl<'a> std::convert::TryFrom<&'a Tagged<Value>> for &'a crate::data::Dictionary {
    type Error = ShellError;

    fn try_from(value: &'a Tagged<Value>) -> Result<&'a crate::data::Dictionary, ShellError> {
        match value.item() {
            Value::Row(d) => Ok(d),
            v => Err(ShellError::type_error(
                "Dictionary",
                value.copy_tag(v.type_name()),
            )),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub enum Switch {
    Present,
    Absent,
}

impl std::convert::TryFrom<Option<&Tagged<Value>>> for Switch {
    type Error = ShellError;

    fn try_from(value: Option<&Tagged<Value>>) -> Result<Switch, ShellError> {
        match value {
            None => Ok(Switch::Absent),
            Some(value) => match value.item() {
                Value::Primitive(Primitive::Boolean(true)) => Ok(Switch::Present),
                v => Err(ShellError::type_error(
                    "Boolean",
                    value.copy_tag(v.type_name()),
                )),
            },
        }
    }
}

impl Tagged<Value> {
    pub(crate) fn debug(&self) -> ValueDebug<'_> {
        ValueDebug { value: self }
    }

    pub fn as_column_path(&self) -> Result<Tagged<Vec<Tagged<String>>>, ShellError> {
        let mut out: Vec<Tagged<String>> = vec![];

        match &self.item {
            Value::Table(table) => {
                for item in table {
                    out.push(item.as_string()?.tagged(item.tag));
                }
            }

            other => {
                return Err(ShellError::type_error(
                    "column name",
                    other.type_name().tagged(self.tag),
                ))
            }
        }

        Ok(out.tagged(self.tag))
    }
}

impl Value {
    pub(crate) fn type_name(&self) -> String {
        match self {
            Value::Primitive(p) => p.type_name(),
            Value::Row(_) => format!("row"),
            Value::Table(_) => format!("list"),
            Value::Block(_) => format!("block"),
        }
    }

    // TODO: This is basically a legacy construct, I think
    pub fn data_descriptors(&self) -> Vec<String> {
        match self {
            Value::Primitive(_) => vec![],
            Value::Row(o) => o
                .entries
                .keys()
                .into_iter()
                .map(|x| x.to_string())
                .collect(),
            Value::Block(_) => vec![],
            Value::Table(_) => vec![],
        }
    }

    pub(crate) fn get_data_by_key(&self, name: &str) -> Option<&Tagged<Value>> {
        match self {
            Value::Row(o) => o.get_data_by_key(name),
            Value::Table(l) => {
                for item in l {
                    match item {
                        Tagged {
                            item: Value::Row(o),
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

    pub fn get_data_by_column_path(
        &self,
        tag: Tag,
        path: &Vec<Tagged<String>>,
    ) -> Option<Tagged<&Value>> {
        let mut current = self;
        for p in path {
            match current.get_data_by_key(p) {
                Some(v) => current = v,
                None => return None,
            }
        }

        Some(Tagged::from_item(current, tag))
    }

    pub fn get_data_by_path(&self, tag: Tag, path: &str) -> Option<Tagged<&Value>> {
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
        &self,
        tag: Tag,
        path: &str,
        new_value: Value,
    ) -> Option<Tagged<Value>> {
        let mut new_obj = self.clone();

        let split_path: Vec<_> = path.split(".").collect();

        if let Value::Row(ref mut o) = new_obj {
            let mut current = o;

            if split_path.len() == 1 {
                // Special case for inserting at the top level
                current
                    .entries
                    .insert(path.to_string(), Tagged::from_item(new_value, tag));
                return Some(Tagged::from_item(new_obj, tag));
            }

            for idx in 0..split_path.len() {
                match current.entries.get_mut(split_path[idx]) {
                    Some(next) => {
                        if idx == (split_path.len() - 2) {
                            match &mut next.item {
                                Value::Row(o) => {
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
                                Value::Row(ref mut o) => {
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

    pub fn insert_data_at_column_path(
        &self,
        tag: Tag,
        split_path: &Vec<Tagged<String>>,
        new_value: Value,
    ) -> Option<Tagged<Value>> {
        let mut new_obj = self.clone();

        if let Value::Row(ref mut o) = new_obj {
            let mut current = o;

            if split_path.len() == 1 {
                // Special case for inserting at the top level
                current.entries.insert(
                    split_path[0].item.clone(),
                    Tagged::from_item(new_value, tag),
                );
                return Some(Tagged::from_item(new_obj, tag));
            }

            for idx in 0..split_path.len() {
                match current.entries.get_mut(&split_path[idx].item) {
                    Some(next) => {
                        if idx == (split_path.len() - 2) {
                            match &mut next.item {
                                Value::Row(o) => {
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
                                Value::Row(ref mut o) => {
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
        &self,
        tag: Tag,
        path: &str,
        replaced_value: Value,
    ) -> Option<Tagged<Value>> {
        let mut new_obj = self.clone();

        let split_path: Vec<_> = path.split(".").collect();

        if let Value::Row(ref mut o) = new_obj {
            let mut current = o;
            for idx in 0..split_path.len() {
                match current.entries.get_mut(split_path[idx]) {
                    Some(next) => {
                        if idx == (split_path.len() - 1) {
                            *next = Tagged::from_item(replaced_value, tag);
                            return Some(Tagged::from_item(new_obj, tag));
                        } else {
                            match next.item {
                                Value::Row(ref mut o) => {
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

    pub fn replace_data_at_column_path(
        &self,
        tag: Tag,
        split_path: &Vec<Tagged<String>>,
        replaced_value: Value,
    ) -> Option<Tagged<Value>> {
        let mut new_obj = self.clone();

        if let Value::Row(ref mut o) = new_obj {
            let mut current = o;
            for idx in 0..split_path.len() {
                match current.entries.get_mut(&split_path[idx].item) {
                    Some(next) => {
                        if idx == (split_path.len() - 1) {
                            *next = Tagged::from_item(replaced_value, tag);
                            return Some(Tagged::from_item(new_obj, tag));
                        } else {
                            match next.item {
                                Value::Row(ref mut o) => {
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

    pub fn get_data(&self, desc: &String) -> MaybeOwned<'_, Value> {
        match self {
            p @ Value::Primitive(_) => MaybeOwned::Borrowed(p),
            Value::Row(o) => o.get_data(desc),
            Value::Block(_) => MaybeOwned::Owned(Value::nothing()),
            Value::Table(_) => MaybeOwned::Owned(Value::nothing()),
        }
    }

    pub(crate) fn format_leaf(&self, desc: Option<&String>) -> String {
        match self {
            Value::Primitive(p) => p.format(desc),
            Value::Block(b) => itertools::join(
                b.expressions
                    .iter()
                    .map(|e| e.source(&b.source).to_string()),
                "; ",
            ),
            Value::Row(_) => format!("[table: 1 row]"),
            Value::Table(l) => format!(
                "[table: {} {}]",
                l.len(),
                if l.len() == 1 { "row" } else { "rows" }
            ),
        }
    }

    pub(crate) fn style_leaf(&self) -> &'static str {
        match self {
            Value::Primitive(p) => p.style(),
            _ => "",
        }
    }

    pub(crate) fn compare(
        &self,
        operator: &Operator,
        other: &Value,
    ) -> Result<bool, (String, String)> {
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

    pub(crate) fn as_string(&self) -> Result<String, ShellError> {
        match self {
            Value::Primitive(Primitive::String(s)) => Ok(s.clone()),
            Value::Primitive(Primitive::Boolean(x)) => Ok(format!("{}", x)),
            Value::Primitive(Primitive::Decimal(x)) => Ok(format!("{}", x)),
            Value::Primitive(Primitive::Int(x)) => Ok(format!("{}", x)),
            Value::Primitive(Primitive::Bytes(x)) => Ok(format!("{}", x)),
            Value::Primitive(Primitive::Path(x)) => Ok(format!("{}", x.display())),
            // TODO: this should definitely be more general with better errors
            other => Err(ShellError::string(format!(
                "Expected string, got {:?}",
                other
            ))),
        }
    }

    pub(crate) fn is_true(&self) -> bool {
        match self {
            Value::Primitive(Primitive::Boolean(true)) => true,
            _ => false,
        }
    }

    pub fn string(s: impl Into<String>) -> Value {
        Value::Primitive(Primitive::String(s.into()))
    }

    pub fn pattern(s: impl Into<String>) -> Value {
        Value::Primitive(Primitive::String(s.into()))
    }

    pub fn path(s: impl Into<PathBuf>) -> Value {
        Value::Primitive(Primitive::Path(s.into()))
    }

    pub fn bytes(s: impl Into<u64>) -> Value {
        Value::Primitive(Primitive::Bytes(s.into()))
    }

    pub fn int(s: impl Into<BigInt>) -> Value {
        Value::Primitive(Primitive::Int(s.into()))
    }

    pub fn decimal(s: impl Into<BigDecimal>) -> Value {
        Value::Primitive(Primitive::Decimal(s.into()))
    }

    pub fn binary(binary: Vec<u8>) -> Value {
        Value::Primitive(Primitive::Binary(binary))
    }

    pub fn number(s: impl Into<Number>) -> Value {
        let num = s.into();

        match num {
            Number::Int(int) => Value::int(int),
            Number::Decimal(decimal) => Value::decimal(decimal),
        }
    }

    pub fn boolean(s: impl Into<bool>) -> Value {
        Value::Primitive(Primitive::Boolean(s.into()))
    }

    pub fn system_date(s: SystemTime) -> Value {
        Value::Primitive(Primitive::Date(s.into()))
    }

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

impl Tagged<Value> {
    pub(crate) fn as_path(&self) -> Result<PathBuf, ShellError> {
        match self.item() {
            Value::Primitive(Primitive::Path(path)) => Ok(path.clone()),
            other => Err(ShellError::type_error(
                "Path",
                other.type_name().tagged(self.tag()),
            )),
        }
    }
}

pub(crate) fn select_fields(obj: &Value, fields: &[String], tag: impl Into<Tag>) -> Tagged<Value> {
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

pub(crate) fn reject_fields(obj: &Value, fields: &[String], tag: impl Into<Tag>) -> Tagged<Value> {
    let mut out = TaggedDictBuilder::new(tag);

    let descs = obj.data_descriptors();

    for desc in descs {
        if fields.iter().any(|field| *field == desc) {
            continue;
        } else {
            out.insert(desc.clone(), obj.get_data(&desc).borrow().clone())
        }
    }

    out.into_tagged_value()
}

enum CompareValues {
    Ints(BigInt, BigInt),
    Decimals(BigDecimal, BigDecimal),
    String(String, String),
}

impl CompareValues {
    fn compare(&self) -> std::cmp::Ordering {
        match self {
            CompareValues::Ints(left, right) => left.cmp(right),
            CompareValues::Decimals(left, right) => left.cmp(right),
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
        (Int(left), Int(right)) => CompareValues::Ints(left.clone(), right.clone()),
        (Int(left), Decimal(right)) => {
            CompareValues::Decimals(BigDecimal::zero() + left, right.clone())
        }
        (Int(left), Bytes(right)) => CompareValues::Ints(left.clone(), BigInt::from(*right)),
        (Decimal(left), Decimal(right)) => CompareValues::Decimals(left.clone(), right.clone()),
        (Decimal(left), Int(right)) => {
            CompareValues::Decimals(left.clone(), BigDecimal::zero() + right)
        }
        (Decimal(left), Bytes(right)) => {
            CompareValues::Decimals(left.clone(), BigDecimal::from(*right))
        }
        (Bytes(left), Int(right)) => CompareValues::Ints(BigInt::from(*left), right.clone()),
        (Bytes(left), Decimal(right)) => {
            CompareValues::Decimals(BigDecimal::from(*left), right.clone())
        }
        (String(left), String(right)) => CompareValues::String(left.clone(), right.clone()),
        _ => return Err((left.type_name(), right.type_name())),
    })
}
