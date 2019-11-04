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
use crate::Text;
use chrono::{DateTime, Utc};
use chrono_humanize::Humanize;
use derive_new::new;
use indexmap::IndexMap;
use log::trace;
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
                        .to_string(),
                );

                for member in members {
                    f.push_str(".");
                    f.push_str(&member.to_string())
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
    pub fn invoke(&self, value: &Tagged<Value>) -> Result<Tagged<Value>, ShellError> {
        let scope = Scope::new(value.clone());

        if self.expressions.len() == 0 {
            return Ok(Value::nothing().tagged(&self.tag));
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

    // Errors are a type of value too
    Error(ShellError),

    Block(Block),
}

impl ShellTypeName for Value {
    fn type_name(&self) -> &'static str {
        match self {
            Value::Primitive(p) => p.type_name(),
            Value::Row(_) => "row",
            Value::Table(_) => "table",
            Value::Error(_) => "error",
            Value::Block(_) => "block",
        }
    }
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

impl Tagged<Value> {
    pub fn tagged_type_name(&self) -> Tagged<String> {
        let name = self.type_name().to_string();
        name.tagged(self.tag())
    }
}

impl Tagged<&Value> {
    pub fn tagged_type_name(&self) -> Tagged<String> {
        let name = self.type_name().to_string();
        name.tagged(self.tag())
    }
}

impl std::convert::TryFrom<&Tagged<Value>> for Block {
    type Error = ShellError;

    fn try_from(value: &Tagged<Value>) -> Result<Block, ShellError> {
        match value.item() {
            Value::Block(block) => Ok(block.clone()),
            v => Err(ShellError::type_error(
                "Block",
                v.type_name().spanned(value.span()),
            )),
        }
    }
}

impl std::convert::TryFrom<&Tagged<Value>> for i64 {
    type Error = ShellError;

    fn try_from(value: &Tagged<Value>) -> Result<i64, ShellError> {
        match value.item() {
            Value::Primitive(Primitive::Int(int)) => {
                int.tagged(&value.tag).coerce_into("converting to i64")
            }
            v => Err(ShellError::type_error(
                "Integer",
                v.type_name().spanned(value.span()),
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
                v.type_name().spanned(value.span()),
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
                v.type_name().spanned(value.span()),
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
                v.type_name().spanned(value.span()),
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
                    v.type_name().spanned(value.span()),
                )),
            },
        }
    }
}

impl Value {
    pub fn data_descriptors(&self) -> Vec<String> {
        match self {
            Value::Primitive(_) => vec![],
            Value::Row(columns) => columns
                .entries
                .keys()
                .into_iter()
                .map(|x| x.to_string())
                .collect(),
            Value::Block(_) => vec![],
            Value::Table(_) => vec![],
            Value::Error(_) => vec![],
        }
    }

    pub fn get_data(&self, desc: &String) -> MaybeOwned<'_, Value> {
        match self {
            p @ Value::Primitive(_) => MaybeOwned::Borrowed(p),
            Value::Row(o) => o.get_data(desc),
            Value::Block(_) => MaybeOwned::Owned(Value::nothing()),
            Value::Table(_) => MaybeOwned::Owned(Value::nothing()),
            Value::Error(_) => MaybeOwned::Owned(Value::nothing()),
        }
    }

    pub(crate) fn format_type(&self, width: usize) -> String {
        TypeShape::from_value(self).colored_string(width)
    }

    pub(crate) fn format_leaf(&self) -> DebugDocBuilder {
        InlineShape::from_value(self).format().pretty_debug()
    }

    pub(crate) fn format_for_column(&self, column: impl Into<Column>) -> DebugDocBuilder {
        InlineShape::from_value(self)
            .format_for_column(column)
            .pretty_debug()
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

    pub(crate) fn is_true(&self) -> bool {
        match self {
            Value::Primitive(Primitive::Boolean(true)) => true,
            _ => false,
        }
    }

    pub(crate) fn is_error(&self) -> bool {
        match self {
            Value::Error(_err) => true,
            _ => false,
        }
    }

    pub(crate) fn expect_error(&self) -> ShellError {
        match self {
            Value::Error(err) => err.clone(),
            _ => panic!("Don't call expect_error without first calling is_error"),
        }
    }

    #[allow(unused)]
    pub fn row(entries: IndexMap<String, Tagged<Value>>) -> Value {
        Value::Row(entries.into())
    }

    pub fn table(list: &Vec<Tagged<Value>>) -> Value {
        Value::Table(list.to_vec())
    }

    pub fn string(s: impl Into<String>) -> Value {
        Value::Primitive(Primitive::String(s.into()))
    }

    pub fn column_path(s: Vec<impl Into<PathMember>>) -> Value {
        Value::Primitive(Primitive::ColumnPath(ColumnPath::new(
            s.into_iter().map(|p| p.into()).collect(),
        )))
    }

    pub fn int(i: impl Into<BigInt>) -> Value {
        Value::Primitive(Primitive::Int(i.into()))
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

    pub fn duration(secs: u64) -> Value {
        Value::Primitive(Primitive::Duration(secs))
    }

    pub fn system_date(s: SystemTime) -> Value {
        Value::Primitive(Primitive::Date(s.into()))
    }

    pub fn date_from_str(s: Tagged<&str>) -> Result<Value, ShellError> {
        let date = DateTime::parse_from_rfc3339(s.item).map_err(|err| {
            ShellError::labeled_error(
                &format!("Date parse error: {}", err),
                "original value",
                s.tag,
            )
        })?;

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
            Value::Primitive(Primitive::String(path_str)) => Ok(PathBuf::from(&path_str).clone()),
            other => Err(ShellError::type_error(
                "Path",
                other.type_name().spanned(self.span()),
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
    match (left, right) {
        (Value::Primitive(left), Value::Primitive(right)) => coerce_compare_primitive(left, right),

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

    use crate::data::meta::*;
    use crate::parser::hir::path::PathMember;
    use crate::ColumnPath as ColumnPathValue;
    use crate::ShellError;
    use crate::Value;
    use indexmap::IndexMap;
    use num_bigint::BigInt;

    fn string(input: impl Into<String>) -> Tagged<Value> {
        Value::string(input.into()).tagged_unknown()
    }

    fn int(input: impl Into<BigInt>) -> Tagged<Value> {
        Value::int(input.into()).tagged_unknown()
    }

    fn row(entries: IndexMap<String, Tagged<Value>>) -> Tagged<Value> {
        Value::row(entries).tagged_unknown()
    }

    fn table(list: &Vec<Tagged<Value>>) -> Tagged<Value> {
        Value::table(list).tagged_unknown()
    }

    fn error_callback(
        reason: &'static str,
    ) -> impl FnOnce((&Value, &PathMember, ShellError)) -> ShellError {
        move |(_obj_source, _column_path_tried, _err)| ShellError::unimplemented(reason)
    }

    fn column_path(paths: &Vec<Tagged<Value>>) -> Tagged<ColumnPathValue> {
        table(&paths.iter().cloned().collect())
            .as_column_path()
            .unwrap()
    }

    #[test]
    fn gets_matching_field_from_a_row() {
        let row = Value::row(indexmap! {
            "amigos".into() => table(&vec![string("andres"),string("jonathan"),string("yehuda")])
        });

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

        let value = Value::row(indexmap! {
            "package".into() =>
                row(indexmap! {
                    "name".into()    =>     string("nu"),
                    "version".into() =>  string("0.4.0")
                })
        });

        assert_eq!(
            *value
                .tagged(tag)
                .get_data_by_column_path(&field_path, Box::new(error_callback("package.version")))
                .unwrap(),
            version
        )
    }

    #[test]
    fn gets_first_matching_field_from_rows_with_same_field_inside_a_table() {
        let field_path = column_path(&vec![string("package"), string("authors"), string("name")]);

        let (_, tag) = string("Andrés N. Robalino").into_parts();

        let value = Value::row(indexmap! {
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
                .tagged(tag)
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

        let value = Value::row(indexmap! {
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
                .tagged(tag)
                .get_data_by_column_path(&field_path, Box::new(error_callback("package.authors.0")))
                .unwrap(),
            Value::row(indexmap! {
                "name".into() => string("Andrés N. Robalino")
            })
        );
    }

    #[test]
    fn column_path_that_contains_just_a_number_gets_a_row_from_a_row() {
        let field_path = column_path(&vec![string("package"), string("authors"), string("0")]);

        let (_, tag) = string("Andrés N. Robalino").into_parts();

        let value = Value::row(indexmap! {
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
                .tagged(tag)
                .get_data_by_column_path(
                    &field_path,
                    Box::new(error_callback("package.authors.\"0\""))
                )
                .unwrap(),
            Value::row(indexmap! {
                "name".into() => string("Andrés N. Robalino")
            })
        );
    }

    #[test]
    fn replaces_matching_field_from_a_row() {
        let field_path = column_path(&vec![string("amigos")]);

        let sample = Value::row(indexmap! {
            "amigos".into() => table(&vec![
                string("andres"),
                string("jonathan"),
                string("yehuda"),
            ]),
        });

        let (replacement, tag) = string("jonas").into_parts();

        let actual = sample
            .tagged(tag)
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

        let sample = Value::row(indexmap! {
            "package".into() => row(indexmap! {
                "authors".into() => row(indexmap! {
                    "los.3.mosqueteros".into() => table(&vec![string("andres::yehuda::jonathan")]),
                    "los.3.amigos".into() => table(&vec![string("andres::yehuda::jonathan")]),
                    "los.3.caballeros".into() => table(&vec![string("andres::yehuda::jonathan")])
                })
            })
        });

        let (replacement, tag) = table(&vec![string("yehuda::jonathan::andres")]).into_parts();

        let actual = sample
            .tagged(tag.clone())
            .replace_data_at_column_path(&field_path, replacement.clone())
            .unwrap();

        assert_eq!(
            actual,
            Value::row(indexmap! {
            "package".into() => row(indexmap! {
                "authors".into() => row(indexmap! {
                    "los.3.mosqueteros".into() => table(&vec![string("andres::yehuda::jonathan")]),
                    "los.3.amigos".into()      => table(&vec![string("andres::yehuda::jonathan")]),
                    "los.3.caballeros".into()  => replacement.tagged(&tag)})})})
            .tagged(tag)
        );
    }
    #[test]
    fn replaces_matching_field_from_rows_inside_a_table() {
        let field_path = column_path(&vec![
            string("shell_policy"),
            string("releases"),
            string("nu.version.arepa"),
        ]);

        let sample = Value::row(indexmap! {
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

        let (replacement, tag) = row(indexmap! {
            "code".into() => string("0.5.0"),
            "tag_line".into() => string("CABALLEROS")
        })
        .into_parts();

        let actual = sample
            .tagged(tag.clone())
            .replace_data_at_column_path(&field_path, replacement.clone())
            .unwrap();

        assert_eq!(
            actual,
            Value::row(indexmap! {
                "shell_policy".into() => row(indexmap! {
                    "releases".into() => table(&vec![
                        row(indexmap! {
                            "nu.version.arepa".into() => replacement.tagged(&tag)
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
            }).tagged(&tag)
        );
    }
}
