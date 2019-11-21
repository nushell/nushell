mod debug;
mod property_get;
pub(crate) mod shape;

use crate::context::CommandRegistry;
use crate::data::base::shape::{Column, InlineShape, TypeShape};
use crate::data::TaggedDictBuilder;
use crate::errors::ShellError;
use crate::evaluate::{evaluate_baseline_expr, Scope};
use crate::parser::hir::path::{ColumnPath, PathMember};
use crate::parser::{hir, Operator};
use crate::prelude::*;
use chrono::{DateTime, Utc};
use chrono_humanize::Humanize;
use derive_new::new;
use indexmap::IndexMap;
use log::trace;
use nu_source::{AnchorLocation, PrettyDebug, SpannedItem, Tagged, TaggedItem, Text};
use serde::{Deserialize, Serialize};
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
    ColumnPath(ColumnPath),
    Pattern(String),
    Boolean(bool),
    Date(DateTime<Utc>),
    Duration(u64), // Duration in seconds
    Path(PathBuf),
    #[serde(with = "serde_bytes")]
    Binary(Vec<u8>),

    // Stream markers (used as bookend markers rather than actual values)
    BeginningOfStream,
    EndOfStream,
}

impl ShellTypeName for Primitive {
    fn type_name(&self) -> &'static str {
        match self {
            Primitive::Nothing => "nothing",
            Primitive::Int(_) => "integer",
            Primitive::Decimal(_) => "decimal",
            Primitive::Bytes(_) => "bytes",
            Primitive::String(_) => "string",
            Primitive::ColumnPath(_) => "column path",
            Primitive::Pattern(_) => "pattern",
            Primitive::Boolean(_) => "boolean",
            Primitive::Date(_) => "date",
            Primitive::Duration(_) => "duration",
            Primitive::Path(_) => "file path",
            Primitive::Binary(_) => "binary",
            Primitive::BeginningOfStream => "marker<beginning of stream>",
            Primitive::EndOfStream => "marker<end of stream>",
        }
    }
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
            Primitive::Duration(sec) => format_duration(*sec),
            Primitive::Int(i) => format!("{}", i),
            Primitive::Decimal(decimal) => format!("{}", decimal),
            Primitive::Pattern(s) => format!("{}", s),
            Primitive::String(s) => format!("{}", s),
            Primitive::ColumnPath(p) => {
                let mut members = p.iter();
                let mut f = String::new();

                f.push_str(
                    &members
                        .next()
                        .expect("BUG: column path with zero members")
                        .display(),
                );

                for member in members {
                    f.push_str(".");
                    f.push_str(&member.display())
                }

                f
            }
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

fn format_duration(sec: u64) -> String {
    let (minutes, seconds) = (sec / 60, sec % 60);
    let (hours, minutes) = (minutes / 60, minutes % 60);
    let (days, hours) = (hours / 24, hours % 24);

    match (days, hours, minutes, seconds) {
        (0, 0, 0, 1) => format!("1 sec"),
        (0, 0, 0, s) => format!("{} secs", s),
        (0, 0, m, s) => format!("{}:{:02}", m, s),
        (0, h, m, s) => format!("{}:{:02}:{:02}", h, m, s),
        (d, h, m, s) => format!("{}:{:02}:{:02}:{:02}", d, h, m, s),
    }
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone, new, Serialize)]
pub struct Operation {
    pub(crate) left: Value,
    pub(crate) operator: Operator,
    pub(crate) right: Value,
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone, Hash, Serialize, Deserialize, new)]
pub struct Block {
    pub(crate) expressions: Vec<hir::Expression>,
    pub(crate) source: Text,
    pub(crate) tag: Tag,
}

impl Block {
    pub fn invoke(&self, value: &Value) -> Result<Value, ShellError> {
        let scope = Scope::new(value.clone());

        if self.expressions.len() == 0 {
            return Ok(UntaggedValue::nothing().into_value(&self.tag));
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
pub enum UntaggedValue {
    Primitive(Primitive),
    Row(crate::data::Dictionary),
    Table(Vec<Value>),

    // Errors are a type of value too
    Error(ShellError),

    Block(Block),
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct Value {
    pub value: UntaggedValue,
    pub tag: Tag,
}

impl std::ops::Deref for Value {
    type Target = UntaggedValue;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl Into<UntaggedValue> for Value {
    fn into(self) -> UntaggedValue {
        self.value
    }
}

impl<'a> Into<&'a UntaggedValue> for &'a Value {
    fn into(self) -> &'a UntaggedValue {
        &self.value
    }
}

impl HasSpan for Value {
    fn span(&self) -> Span {
        self.tag.span
    }
}

impl ShellTypeName for Value {
    fn type_name(&self) -> &'static str {
        ShellTypeName::type_name(&self.value)
    }
}

impl ShellTypeName for UntaggedValue {
    fn type_name(&self) -> &'static str {
        match &self {
            UntaggedValue::Primitive(p) => p.type_name(),
            UntaggedValue::Row(_) => "row",
            UntaggedValue::Table(_) => "table",
            UntaggedValue::Error(_) => "error",
            UntaggedValue::Block(_) => "block",
        }
    }
}

impl Into<UntaggedValue> for Number {
    fn into(self) -> UntaggedValue {
        match self {
            Number::Int(int) => UntaggedValue::int(int),
            Number::Decimal(decimal) => UntaggedValue::decimal(decimal),
        }
    }
}

impl Into<UntaggedValue> for &Number {
    fn into(self) -> UntaggedValue {
        match self {
            Number::Int(int) => UntaggedValue::int(int.clone()),
            Number::Decimal(decimal) => UntaggedValue::decimal(decimal.clone()),
        }
    }
}

impl Value {
    pub fn anchor(&self) -> Option<AnchorLocation> {
        self.tag.anchor()
    }

    pub fn anchor_name(&self) -> Option<String> {
        self.tag.anchor_name()
    }

    pub fn tag(&self) -> Tag {
        self.tag.clone()
    }

    pub fn into_parts(self) -> (UntaggedValue, Tag) {
        (self.value, self.tag)
    }

    pub(crate) fn as_path(&self) -> Result<PathBuf, ShellError> {
        match &self.value {
            UntaggedValue::Primitive(Primitive::Path(path)) => Ok(path.clone()),
            UntaggedValue::Primitive(Primitive::String(path_str)) => {
                Ok(PathBuf::from(&path_str).clone())
            }
            _ => Err(ShellError::type_error("Path", self.spanned_type_name())),
        }
    }

    pub fn tagged_type_name(&self) -> Tagged<String> {
        let name = self.type_name().to_string();
        name.tagged(self.tag.clone())
    }

    pub(crate) fn compare(
        &self,
        operator: &Operator,
        other: &Value,
    ) -> Result<bool, (&'static str, &'static str)> {
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
}

impl PrettyDebug for &Value {
    fn pretty(&self) -> DebugDocBuilder {
        PrettyDebug::pretty(*self)
    }
}

impl PrettyDebug for Value {
    fn pretty(&self) -> DebugDocBuilder {
        match &self.value {
            UntaggedValue::Primitive(p) => p.pretty(),
            UntaggedValue::Row(row) => row.pretty_builder().nest(1).group().into(),
            UntaggedValue::Table(table) => {
                b::delimit("[", b::intersperse(table, b::space()), "]").nest()
            }
            UntaggedValue::Error(_) => b::error("error"),
            UntaggedValue::Block(_) => b::opaque("block"),
        }
    }
}

impl std::convert::TryFrom<&Value> for Block {
    type Error = ShellError;

    fn try_from(value: &Value) -> Result<Block, ShellError> {
        match &value.value {
            UntaggedValue::Block(block) => Ok(block.clone()),
            _ => Err(ShellError::type_error(
                "Block",
                value.type_name().spanned(value.tag.span),
            )),
        }
    }
}

impl std::convert::TryFrom<&Value> for i64 {
    type Error = ShellError;

    fn try_from(value: &Value) -> Result<i64, ShellError> {
        match &value.value {
            UntaggedValue::Primitive(Primitive::Int(int)) => {
                int.tagged(&value.tag).coerce_into("converting to i64")
            }
            _ => Err(ShellError::type_error("Integer", value.spanned_type_name())),
        }
    }
}

impl std::convert::TryFrom<&Value> for String {
    type Error = ShellError;

    fn try_from(value: &Value) -> Result<String, ShellError> {
        match &value.value {
            UntaggedValue::Primitive(Primitive::String(s)) => Ok(s.clone()),
            _ => Err(ShellError::type_error("String", value.spanned_type_name())),
        }
    }
}

impl std::convert::TryFrom<&Value> for Vec<u8> {
    type Error = ShellError;

    fn try_from(value: &Value) -> Result<Vec<u8>, ShellError> {
        match &value.value {
            UntaggedValue::Primitive(Primitive::Binary(b)) => Ok(b.clone()),
            _ => Err(ShellError::type_error("Binary", value.spanned_type_name())),
        }
    }
}

impl<'a> std::convert::TryFrom<&'a Value> for &'a crate::data::Dictionary {
    type Error = ShellError;

    fn try_from(value: &'a Value) -> Result<&'a crate::data::Dictionary, ShellError> {
        match &value.value {
            UntaggedValue::Row(d) => Ok(d),
            _ => Err(ShellError::type_error(
                "Dictionary",
                value.spanned_type_name(),
            )),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub enum Switch {
    Present,
    Absent,
}

impl std::convert::TryFrom<Option<&Value>> for Switch {
    type Error = ShellError;

    fn try_from(value: Option<&Value>) -> Result<Switch, ShellError> {
        match value {
            None => Ok(Switch::Absent),
            Some(value) => match &value.value {
                UntaggedValue::Primitive(Primitive::Boolean(true)) => Ok(Switch::Present),
                _ => Err(ShellError::type_error("Boolean", value.spanned_type_name())),
            },
        }
    }
}

impl UntaggedValue {
    pub fn into_value(self, tag: impl Into<Tag>) -> Value {
        Value {
            value: self,
            tag: tag.into(),
        }
    }

    pub fn into_untagged_value(self) -> Value {
        Value {
            value: self,
            tag: Tag::unknown(),
        }
    }

    pub fn retag(self, tag: impl Into<Tag>) -> Value {
        Value {
            value: self,
            tag: tag.into(),
        }
    }

    pub fn data_descriptors(&self) -> Vec<String> {
        match self {
            UntaggedValue::Primitive(_) => vec![],
            UntaggedValue::Row(columns) => columns
                .entries
                .keys()
                .into_iter()
                .map(|x| x.to_string())
                .collect(),
            UntaggedValue::Block(_) => vec![],
            UntaggedValue::Table(_) => vec![],
            UntaggedValue::Error(_) => vec![],
        }
    }

    #[allow(unused)]
    pub(crate) fn format_type(&self, width: usize) -> String {
        TypeShape::from_value(self).colored_string(width)
    }

    pub(crate) fn format_leaf(&self) -> DebugDocBuilder {
        InlineShape::from_value(self).format().pretty()
    }

    #[allow(unused)]
    pub(crate) fn format_for_column(&self, column: impl Into<Column>) -> DebugDocBuilder {
        InlineShape::from_value(self)
            .format_for_column(column)
            .pretty()
    }

    pub(crate) fn style_leaf(&self) -> &'static str {
        match self {
            UntaggedValue::Primitive(p) => p.style(),
            _ => "",
        }
    }

    pub(crate) fn is_true(&self) -> bool {
        match self {
            UntaggedValue::Primitive(Primitive::Boolean(true)) => true,
            _ => false,
        }
    }

    pub(crate) fn is_some(&self) -> bool {
        !self.is_none()
    }

    pub(crate) fn is_none(&self) -> bool {
        match self {
            UntaggedValue::Primitive(Primitive::Nothing) => true,
            _ => false,
        }
    }

    pub(crate) fn is_error(&self) -> bool {
        match self {
            UntaggedValue::Error(_err) => true,
            _ => false,
        }
    }

    pub(crate) fn expect_error(&self) -> ShellError {
        match self {
            UntaggedValue::Error(err) => err.clone(),
            _ => panic!("Don't call expect_error without first calling is_error"),
        }
    }

    pub fn expect_string(&self) -> &str {
        match self {
            UntaggedValue::Primitive(Primitive::String(string)) => &string[..],
            _ => panic!("expect_string assumes that the value must be a string"),
        }
    }

    #[allow(unused)]
    pub fn row(entries: IndexMap<String, Value>) -> UntaggedValue {
        UntaggedValue::Row(entries.into())
    }

    pub fn table(list: &Vec<Value>) -> UntaggedValue {
        UntaggedValue::Table(list.to_vec())
    }

    pub fn string(s: impl Into<String>) -> UntaggedValue {
        UntaggedValue::Primitive(Primitive::String(s.into()))
    }

    pub fn column_path(s: Vec<impl Into<PathMember>>) -> UntaggedValue {
        UntaggedValue::Primitive(Primitive::ColumnPath(ColumnPath::new(
            s.into_iter().map(|p| p.into()).collect(),
        )))
    }

    pub fn int(i: impl Into<BigInt>) -> UntaggedValue {
        UntaggedValue::Primitive(Primitive::Int(i.into()))
    }

    pub fn pattern(s: impl Into<String>) -> UntaggedValue {
        UntaggedValue::Primitive(Primitive::String(s.into()))
    }

    pub fn path(s: impl Into<PathBuf>) -> UntaggedValue {
        UntaggedValue::Primitive(Primitive::Path(s.into()))
    }

    pub fn bytes(s: impl Into<u64>) -> UntaggedValue {
        UntaggedValue::Primitive(Primitive::Bytes(s.into()))
    }

    pub fn decimal(s: impl Into<BigDecimal>) -> UntaggedValue {
        UntaggedValue::Primitive(Primitive::Decimal(s.into()))
    }

    pub fn binary(binary: Vec<u8>) -> UntaggedValue {
        UntaggedValue::Primitive(Primitive::Binary(binary))
    }

    pub fn number(s: impl Into<Number>) -> UntaggedValue {
        let num = s.into();

        match num {
            Number::Int(int) => UntaggedValue::int(int),
            Number::Decimal(decimal) => UntaggedValue::decimal(decimal),
        }
    }

    pub fn boolean(s: impl Into<bool>) -> UntaggedValue {
        UntaggedValue::Primitive(Primitive::Boolean(s.into()))
    }

    pub fn duration(secs: u64) -> UntaggedValue {
        UntaggedValue::Primitive(Primitive::Duration(secs))
    }

    pub fn system_date(s: SystemTime) -> UntaggedValue {
        UntaggedValue::Primitive(Primitive::Date(s.into()))
    }

    pub fn date_from_str(s: Tagged<&str>) -> Result<UntaggedValue, ShellError> {
        let date = DateTime::parse_from_rfc3339(s.item).map_err(|err| {
            ShellError::labeled_error(
                &format!("Date parse error: {}", err),
                "original value",
                s.tag,
            )
        })?;

        let date = date.with_timezone(&chrono::offset::Utc);

        Ok(UntaggedValue::Primitive(Primitive::Date(date)))
    }

    pub fn nothing() -> UntaggedValue {
        UntaggedValue::Primitive(Primitive::Nothing)
    }
}

pub(crate) fn select_fields(obj: &Value, fields: &[String], tag: impl Into<Tag>) -> Value {
    let mut out = TaggedDictBuilder::new(tag);

    let descs = obj.data_descriptors();

    for field in fields {
        match descs.iter().find(|d| *d == field) {
            None => out.insert_untagged(field, UntaggedValue::nothing()),
            Some(desc) => out.insert_value(desc.clone(), obj.get_data(desc).borrow().clone()),
        }
    }

    out.into_value()
}

pub(crate) fn reject_fields(obj: &Value, fields: &[String], tag: impl Into<Tag>) -> Value {
    let mut out = TaggedDictBuilder::new(tag);

    let descs = obj.data_descriptors();

    for desc in descs {
        if fields.iter().any(|field| *field == desc) {
            continue;
        } else {
            out.insert_value(desc.clone(), obj.get_data(&desc).borrow().clone())
        }
    }

    out.into_value()
}

enum CompareValues {
    Ints(BigInt, BigInt),
    Decimals(BigDecimal, BigDecimal),
    String(String, String),
    Date(DateTime<Utc>, DateTime<Utc>),
    DateDuration(DateTime<Utc>, u64),
}

impl CompareValues {
    fn compare(&self) -> std::cmp::Ordering {
        match self {
            CompareValues::Ints(left, right) => left.cmp(right),
            CompareValues::Decimals(left, right) => left.cmp(right),
            CompareValues::String(left, right) => left.cmp(right),
            CompareValues::Date(left, right) => left.cmp(right),
            CompareValues::DateDuration(left, right) => {
                use std::time::Duration;

                // Create the datetime we're comparing against, as duration is an offset from now
                let right: DateTime<Utc> = (SystemTime::now() - Duration::from_secs(*right)).into();
                right.cmp(left)
            }
        }
    }
}

fn coerce_compare(
    left: &Value,
    right: &Value,
) -> Result<CompareValues, (&'static str, &'static str)> {
    match (&left.value, &right.value) {
        (UntaggedValue::Primitive(left), UntaggedValue::Primitive(right)) => {
            coerce_compare_primitive(left, right)
        }

        _ => Err((left.type_name(), right.type_name())),
    }
}

fn coerce_compare_primitive(
    left: &Primitive,
    right: &Primitive,
) -> Result<CompareValues, (&'static str, &'static str)> {
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
        (Date(left), Date(right)) => CompareValues::Date(left.clone(), right.clone()),
        (Date(left), Duration(right)) => CompareValues::DateDuration(left.clone(), right.clone()),
        _ => return Err((left.type_name(), right.type_name())),
    })
}
#[cfg(test)]
mod tests {

    use super::UntaggedValue;
    use crate::parser::hir::path::PathMember;
    use crate::ColumnPath as ColumnPathValue;
    use crate::ShellError;
    use crate::Value;
    use indexmap::IndexMap;
    use nu_source::*;
    use num_bigint::BigInt;

    fn string(input: impl Into<String>) -> Value {
        UntaggedValue::string(input.into()).into_untagged_value()
    }

    fn int(input: impl Into<BigInt>) -> Value {
        UntaggedValue::int(input.into()).into_untagged_value()
    }

    fn row(entries: IndexMap<String, Value>) -> Value {
        UntaggedValue::row(entries).into_untagged_value()
    }

    fn table(list: &Vec<Value>) -> Value {
        UntaggedValue::table(list).into_untagged_value()
    }

    fn error_callback(
        reason: &'static str,
    ) -> impl FnOnce((&Value, &PathMember, ShellError)) -> ShellError {
        move |(_obj_source, _column_path_tried, _err)| ShellError::unimplemented(reason)
    }

    fn column_path(paths: &Vec<Value>) -> Tagged<ColumnPathValue> {
        table(&paths.iter().cloned().collect())
            .as_column_path()
            .unwrap()
    }

    #[test]
    fn gets_matching_field_from_a_row() {
        let row = UntaggedValue::row(indexmap! {
            "amigos".into() => table(&vec![string("andres"),string("jonathan"),string("yehuda")])
        })
        .into_untagged_value();

        assert_eq!(
            row.get_data_by_key("amigos".spanned_unknown()).unwrap(),
            table(&vec![
                string("andres"),
                string("jonathan"),
                string("yehuda")
            ])
        );
    }

    #[test]
    fn gets_matching_field_from_nested_rows_inside_a_row() {
        let field_path = column_path(&vec![string("package"), string("version")]);

        let (version, tag) = string("0.4.0").into_parts();

        let value = UntaggedValue::row(indexmap! {
            "package".into() =>
                row(indexmap! {
                    "name".into()    =>     string("nu"),
                    "version".into() =>  string("0.4.0")
                })
        });

        assert_eq!(
            *value
                .into_value(tag)
                .get_data_by_column_path(&field_path, Box::new(error_callback("package.version")))
                .unwrap(),
            version
        )
    }

    #[test]
    fn gets_first_matching_field_from_rows_with_same_field_inside_a_table() {
        let field_path = column_path(&vec![string("package"), string("authors"), string("name")]);

        let (_, tag) = string("Andrés N. Robalino").into_parts();

        let value = UntaggedValue::row(indexmap! {
            "package".into() => row(indexmap! {
                "name".into() => string("nu"),
                "version".into() => string("0.4.0"),
                "authors".into() => table(&vec![
                    row(indexmap!{"name".into() => string("Andrés N. Robalino")}),
                    row(indexmap!{"name".into() => string("Jonathan Turner")}),
                    row(indexmap!{"name".into() => string("Yehuda Katz")})
                ])
            })
        });

        assert_eq!(
            value
                .into_value(tag)
                .get_data_by_column_path(
                    &field_path,
                    Box::new(error_callback("package.authors.name"))
                )
                .unwrap(),
            table(&vec![
                string("Andrés N. Robalino"),
                string("Jonathan Turner"),
                string("Yehuda Katz")
            ])
        )
    }

    #[test]
    fn column_path_that_contains_just_a_number_gets_a_row_from_a_table() {
        let field_path = column_path(&vec![string("package"), string("authors"), int(0)]);

        let (_, tag) = string("Andrés N. Robalino").into_parts();

        let value = UntaggedValue::row(indexmap! {
            "package".into() => row(indexmap! {
                "name".into() => string("nu"),
                "version".into() => string("0.4.0"),
                "authors".into() => table(&vec![
                    row(indexmap!{"name".into() => string("Andrés N. Robalino")}),
                    row(indexmap!{"name".into() => string("Jonathan Turner")}),
                    row(indexmap!{"name".into() => string("Yehuda Katz")})
                ])
            })
        });

        assert_eq!(
            *value
                .into_value(tag)
                .get_data_by_column_path(&field_path, Box::new(error_callback("package.authors.0")))
                .unwrap(),
            UntaggedValue::row(indexmap! {
                "name".into() => string("Andrés N. Robalino")
            })
        );
    }

    #[test]
    fn column_path_that_contains_just_a_number_gets_a_row_from_a_row() {
        let field_path = column_path(&vec![string("package"), string("authors"), string("0")]);

        let (_, tag) = string("Andrés N. Robalino").into_parts();

        let value = UntaggedValue::row(indexmap! {
            "package".into() => row(indexmap! {
                "name".into() => string("nu"),
                "version".into() => string("0.4.0"),
                "authors".into() => row(indexmap! {
                    "0".into() => row(indexmap!{"name".into() => string("Andrés N. Robalino")}),
                    "1".into() => row(indexmap!{"name".into() => string("Jonathan Turner")}),
                    "2".into() => row(indexmap!{"name".into() => string("Yehuda Katz")}),
                })
            })
        });

        assert_eq!(
            *value
                .into_value(tag)
                .get_data_by_column_path(
                    &field_path,
                    Box::new(error_callback("package.authors.\"0\""))
                )
                .unwrap(),
            UntaggedValue::row(indexmap! {
                "name".into() => string("Andrés N. Robalino")
            })
        );
    }

    #[test]
    fn replaces_matching_field_from_a_row() {
        let field_path = column_path(&vec![string("amigos")]);

        let sample = UntaggedValue::row(indexmap! {
            "amigos".into() => table(&vec![
                string("andres"),
                string("jonathan"),
                string("yehuda"),
            ]),
        });

        let replacement = string("jonas");

        let actual = sample
            .into_untagged_value()
            .replace_data_at_column_path(&field_path, replacement)
            .unwrap();

        assert_eq!(actual, row(indexmap! {"amigos".into() => string("jonas")}));
    }

    #[test]
    fn replaces_matching_field_from_nested_rows_inside_a_row() {
        let field_path = column_path(&vec![
            string("package"),
            string("authors"),
            string("los.3.caballeros"),
        ]);

        let sample = UntaggedValue::row(indexmap! {
            "package".into() => row(indexmap! {
                "authors".into() => row(indexmap! {
                    "los.3.mosqueteros".into() => table(&vec![string("andres::yehuda::jonathan")]),
                    "los.3.amigos".into() => table(&vec![string("andres::yehuda::jonathan")]),
                    "los.3.caballeros".into() => table(&vec![string("andres::yehuda::jonathan")])
                })
            })
        });

        let replacement = table(&vec![string("yehuda::jonathan::andres")]);
        let tag = replacement.tag.clone();

        let actual = sample
            .into_value(tag.clone())
            .replace_data_at_column_path(&field_path, replacement.clone())
            .unwrap();

        assert_eq!(
            actual,
            UntaggedValue::row(indexmap! {
            "package".into() => row(indexmap! {
                "authors".into() => row(indexmap! {
                    "los.3.mosqueteros".into() => table(&vec![string("andres::yehuda::jonathan")]),
                    "los.3.amigos".into()      => table(&vec![string("andres::yehuda::jonathan")]),
                    "los.3.caballeros".into()  => replacement.clone()})})})
            .into_value(tag)
        );
    }
    #[test]
    fn replaces_matching_field_from_rows_inside_a_table() {
        let field_path = column_path(&vec![
            string("shell_policy"),
            string("releases"),
            string("nu.version.arepa"),
        ]);

        let sample = UntaggedValue::row(indexmap! {
            "shell_policy".into() => row(indexmap! {
                "releases".into() => table(&vec![
                    row(indexmap! {
                        "nu.version.arepa".into() => row(indexmap! {
                            "code".into() => string("0.4.0"), "tag_line".into() => string("GitHub-era")
                        })
                    }),
                    row(indexmap! {
                        "nu.version.taco".into() => row(indexmap! {
                            "code".into() => string("0.3.0"), "tag_line".into() => string("GitHub-era")
                        })
                    }),
                    row(indexmap! {
                        "nu.version.stable".into() => row(indexmap! {
                            "code".into() => string("0.2.0"), "tag_line".into() => string("GitHub-era")
                        })
                    })
                ])
            })
        });

        let replacement = row(indexmap! {
            "code".into() => string("0.5.0"),
            "tag_line".into() => string("CABALLEROS")
        });
        let tag = replacement.tag.clone();

        let actual = sample
            .into_value(tag.clone())
            .replace_data_at_column_path(&field_path, replacement.clone())
            .unwrap();

        assert_eq!(
            actual,
            UntaggedValue::row(indexmap! {
                "shell_policy".into() => row(indexmap! {
                    "releases".into() => table(&vec![
                        row(indexmap! {
                            "nu.version.arepa".into() => replacement
                        }),
                        row(indexmap! {
                            "nu.version.taco".into() => row(indexmap! {
                                "code".into() => string("0.3.0"), "tag_line".into() => string("GitHub-era")
                            })
                        }),
                        row(indexmap! {
                            "nu.version.stable".into() => row(indexmap! {
                                "code".into() => string("0.2.0"), "tag_line".into() => string("GitHub-era")
                            })
                        })
                    ])
                })
            }).into_value(&tag)
        );
    }
}
