pub mod column_path;
mod convert;
mod debug;
pub mod dict;
pub mod did_you_mean;
pub mod iter;
pub mod primitive;
pub mod range;
mod serde_bigdecimal;
mod serde_bigint;
pub mod value_structure;

use crate::hir;
use crate::type_name::{ShellTypeName, SpannedTypeName};
use crate::value::dict::Dictionary;
use crate::value::iter::{RowValueIter, TableValueIter};
use crate::value::primitive::Primitive;
use crate::value::range::{Range, RangeInclusion};
use crate::ColumnPath;
use bigdecimal::BigDecimal;
use bigdecimal::FromPrimitive;
use chrono::{DateTime, FixedOffset, Utc};
use indexmap::IndexMap;
use nu_errors::ShellError;
use nu_source::{AnchorLocation, HasSpan, Span, Spanned, SpannedItem, Tag};
use num_bigint::BigInt;
use num_traits::ToPrimitive;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::time::SystemTime;

/// The core structured values that flow through a pipeline
#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub enum UntaggedValue {
    /// A primitive (or fundamental) type of values
    Primitive(Primitive),

    /// A table row
    Row(Dictionary),

    /// A full inner (or embedded) table
    Table(Vec<Value>),

    /// An error value that represents an error that occurred as the values in the pipeline were built
    Error(ShellError),

    /// A block of Nu code, eg `{ ls | get name ; echo "done" }` with its captured values
    Block(Box<hir::CapturedBlock>),
}

impl UntaggedValue {
    /// Get the corresponding descriptors (column names) associated with this value
    pub fn data_descriptors(&self) -> Vec<String> {
        match self {
            UntaggedValue::Row(columns) => columns.entries.keys().map(|x| x.to_string()).collect(),
            _ => vec![],
        }
    }

    /// Convert this UntaggedValue to a Value with the given Tag
    pub fn into_value(self, tag: impl Into<Tag>) -> Value {
        Value {
            value: self,
            tag: tag.into(),
        }
    }

    /// Convert this UntaggedValue into a Value with an empty Tag
    pub fn into_untagged_value(self) -> Value {
        Value {
            value: self,
            tag: Tag::unknown(),
        }
    }

    /// Returns true if this value represents boolean true
    pub fn is_true(&self) -> bool {
        matches!(self, UntaggedValue::Primitive(Primitive::Boolean(true)))
    }

    /// Returns true if this value represents a filesize
    pub fn is_filesize(&self) -> bool {
        matches!(self, UntaggedValue::Primitive(Primitive::Filesize(_)))
    }

    /// Returns true if this value represents a duration
    pub fn is_duration(&self) -> bool {
        matches!(self, UntaggedValue::Primitive(Primitive::Duration(_)))
    }

    /// Returns true if this value represents a table
    pub fn is_table(&self) -> bool {
        matches!(self, UntaggedValue::Table(_))
    }

    /// Returns true if this value represents a row
    pub fn is_row(&self) -> bool {
        matches!(self, UntaggedValue::Row(_))
    }

    /// Returns true if this value represents a string
    pub fn is_string(&self) -> bool {
        matches!(self, UntaggedValue::Primitive(Primitive::String(_)))
    }

    /// Returns true if the value represents something other than Nothing
    pub fn is_some(&self) -> bool {
        !self.is_none()
    }

    /// Returns true if the value represents Nothing
    pub fn is_none(&self) -> bool {
        matches!(self, UntaggedValue::Primitive(Primitive::Nothing))
    }

    /// Returns true if the value represents an error
    pub fn is_error(&self) -> bool {
        matches!(self, UntaggedValue::Error(_err))
    }

    /// Expect this value to be an error and return it
    pub fn expect_error(&self) -> ShellError {
        match self {
            UntaggedValue::Error(err) => err.clone(),
            _ => panic!("Don't call expect_error without first calling is_error"),
        }
    }

    /// Expect this value to be a string and return it
    pub fn expect_string(&self) -> &str {
        match self {
            UntaggedValue::Primitive(Primitive::String(string)) => &string[..],
            _ => panic!("expect_string assumes that the value must be a string"),
        }
    }

    /// Expect this value to be an integer and return it
    pub fn expect_int(&self) -> i64 {
        let big_int = match self {
            UntaggedValue::Primitive(Primitive::Int(int)) => Some(int),
            _ => None,
        };

        match big_int.and_then(|i| i.to_i64()) {
            Some(i) => i,
            _ => panic!("expect_int assumes that the value must be a integer"),
        }
    }

    /// Helper for creating row values
    pub fn row(entries: IndexMap<String, Value>) -> UntaggedValue {
        UntaggedValue::Row(entries.into())
    }

    /// Helper for creating table values
    pub fn table(list: &[Value]) -> UntaggedValue {
        UntaggedValue::Table(list.to_vec())
    }

    /// Helper for creating string values
    pub fn string(s: impl Into<String>) -> UntaggedValue {
        UntaggedValue::Primitive(Primitive::String(s.into()))
    }

    /// Helper for creating column-path values
    pub fn column_path(s: &str, span: Span) -> UntaggedValue {
        let s = s.to_string().spanned(span);

        UntaggedValue::Primitive(Primitive::ColumnPath(ColumnPath::build(&s)))
    }

    /// Helper for creating integer values
    pub fn int(i: impl Into<BigInt>) -> UntaggedValue {
        UntaggedValue::Primitive(Primitive::Int(i.into()))
    }

    /// Helper for creating glob pattern values
    pub fn glob_pattern(s: impl Into<String>) -> UntaggedValue {
        UntaggedValue::Primitive(Primitive::String(s.into()))
    }

    /// Helper for creating filepath values
    pub fn filepath(s: impl Into<PathBuf>) -> UntaggedValue {
        UntaggedValue::Primitive(Primitive::FilePath(s.into()))
    }

    /// Helper for creating filesize values
    pub fn filesize(s: impl Into<BigInt>) -> UntaggedValue {
        UntaggedValue::Primitive(Primitive::Filesize(s.into()))
    }

    /// Helper for creating decimal values
    pub fn decimal(s: impl Into<BigDecimal>) -> UntaggedValue {
        UntaggedValue::Primitive(Primitive::Decimal(s.into()))
    }

    /// Helper for creating decimal values
    pub fn decimal_from_float(f: f64, span: Span) -> UntaggedValue {
        let dec = BigDecimal::from_f64(f);

        // BigDecimal doesn't have the concept of inf/NaN so handle manually
        if f.is_sign_negative() && f.is_infinite() {
            UntaggedValue::from("-inf")
        } else if f.is_infinite() {
            UntaggedValue::from("inf")
        } else if f.is_nan() {
            UntaggedValue::from("NaN")
        } else {
            match dec {
                Some(dec) => UntaggedValue::Primitive(Primitive::Decimal(dec)),
                None => UntaggedValue::Error(ShellError::labeled_error(
                    "Can not convert f64 to big decimal",
                    "can not create decimal",
                    span,
                )),
            }
        }
    }

    /// Helper for creating binary (non-text) buffer values
    pub fn binary(binary: Vec<u8>) -> UntaggedValue {
        UntaggedValue::Primitive(Primitive::Binary(binary))
    }

    /// Helper for creating range values
    pub fn range(
        left: (Spanned<Primitive>, RangeInclusion),
        right: (Spanned<Primitive>, RangeInclusion),
    ) -> UntaggedValue {
        UntaggedValue::Primitive(Primitive::Range(Box::new(Range::new(left, right))))
    }

    /// Helper for creating boolean values
    pub fn boolean(b: impl Into<bool>) -> UntaggedValue {
        UntaggedValue::Primitive(Primitive::Boolean(b.into()))
    }

    /// Helper for creating date duration values
    pub fn duration(nanos: impl Into<BigInt>) -> UntaggedValue {
        UntaggedValue::Primitive(Primitive::Duration(nanos.into()))
    }

    /// Helper for creating datatime values
    pub fn system_date(s: SystemTime) -> UntaggedValue {
        let utc: DateTime<Utc> = s.into();
        UntaggedValue::Primitive(Primitive::Date(utc.into()))
    }

    pub fn date(d: impl Into<DateTime<FixedOffset>>) -> UntaggedValue {
        UntaggedValue::Primitive(Primitive::Date(d.into()))
    }

    /// Helper for creating the Nothing value
    pub fn nothing() -> UntaggedValue {
        UntaggedValue::Primitive(Primitive::Nothing)
    }
}

/// The fundamental structured value that flows through the pipeline, with associated metadata
#[derive(Debug, Clone, PartialOrd, Ord, Eq, Serialize, Deserialize)]
pub struct Value {
    pub value: UntaggedValue,
    pub tag: Tag,
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl Hash for Value {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

/// Overload deferencing to give back the UntaggedValue inside of a Value
impl std::ops::Deref for Value {
    type Target = UntaggedValue;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl Value {
    /// Helper to create a Value
    pub fn new(untagged_value: UntaggedValue, the_tag: Tag) -> Self {
        Value {
            value: untagged_value,
            tag: the_tag,
        }
    }

    /// Get the corresponding anchor (originating location) for the Value
    pub fn anchor(&self) -> Option<AnchorLocation> {
        self.tag.anchor()
    }

    /// Get the name (url, filepath, etc) behind an anchor for the Value
    pub fn anchor_name(&self) -> Option<String> {
        self.tag.anchor_name()
    }

    /// Get the metadata for the Value
    pub fn tag(&self) -> Tag {
        self.tag.clone()
    }

    /// View the Value as a string, if possible
    pub fn as_string(&self) -> Result<String, ShellError> {
        match &self.value {
            UntaggedValue::Primitive(Primitive::String(string)) => Ok(string.clone()),
            UntaggedValue::Primitive(Primitive::FilePath(path)) => {
                Ok(path.to_string_lossy().to_string())
            }
            _ => Err(ShellError::type_error("string", self.spanned_type_name())),
        }
    }

    /// View the Value as a FilePath (PathBuf), if possible
    pub fn as_filepath(&self) -> Result<PathBuf, ShellError> {
        match &self.value {
            UntaggedValue::Primitive(Primitive::FilePath(path)) => Ok(path.clone()),
            _ => Err(ShellError::type_error("string", self.spanned_type_name())),
        }
    }

    /// View the Value as a Int (BigInt), if possible
    pub fn as_int(&self) -> Result<BigInt, ShellError> {
        match &self.value {
            UntaggedValue::Primitive(Primitive::Int(n)) => Ok(n.clone()),
            _ => Err(ShellError::type_error("bigint", self.spanned_type_name())),
        }
    }

    /// View the Value as a Filesize (BigInt), if possible
    pub fn as_filesize(&self) -> Result<BigInt, ShellError> {
        match &self.value {
            UntaggedValue::Primitive(Primitive::Filesize(fs)) => Ok(fs.clone()),
            _ => Err(ShellError::type_error("bigint", self.spanned_type_name())),
        }
    }

    /// View the Value as a Duration (BigInt), if possible
    pub fn as_duration(&self) -> Result<BigInt, ShellError> {
        match &self.value {
            UntaggedValue::Primitive(Primitive::Duration(dur)) => Ok(dur.clone()),
            _ => Err(ShellError::type_error("bigint", self.spanned_type_name())),
        }
    }
    /// View the Value as a Decimal (BigDecimal), if possible
    pub fn as_decimal(&self) -> Result<BigDecimal, ShellError> {
        match &self.value {
            UntaggedValue::Primitive(Primitive::Decimal(d)) => Ok(d.clone()),
            _ => Err(ShellError::type_error(
                "bigdecimal",
                self.spanned_type_name(),
            )),
        }
    }

    pub fn convert_to_string(&self) -> String {
        match &self.value {
            UntaggedValue::Primitive(Primitive::String(s)) => s.clone(),
            UntaggedValue::Primitive(Primitive::Date(dt)) => dt.format("%Y-%m-%d").to_string(),
            UntaggedValue::Primitive(Primitive::Boolean(x)) => format!("{}", x),
            UntaggedValue::Primitive(Primitive::Decimal(x)) => format!("{}", x),
            UntaggedValue::Primitive(Primitive::Int(x)) => format!("{}", x),
            UntaggedValue::Primitive(Primitive::Filesize(x)) => format!("{}", x),
            UntaggedValue::Primitive(Primitive::FilePath(x)) => format!("{}", x.display()),
            UntaggedValue::Primitive(Primitive::ColumnPath(path)) => {
                let joined: String = path
                    .iter()
                    .map(|member| member.as_string())
                    .collect::<Vec<String>>()
                    .join(".");

                if joined.contains(' ') {
                    format!("\"{}\"", joined)
                } else {
                    joined
                }
            }

            _ => String::from(""),
        }
    }

    pub fn format(&self, fmt: &str) -> Result<String, ShellError> {
        match &self.value {
            UntaggedValue::Primitive(Primitive::Date(dt)) => Ok(dt.format(fmt).to_string()),
            _ => Err(ShellError::type_error("date", self.spanned_type_name())),
        }
    }

    /// View into the borrowed string contents of a Value, if possible
    pub fn as_forgiving_string(&self) -> Result<&str, ShellError> {
        match &self.value {
            UntaggedValue::Primitive(Primitive::String(string)) => Ok(&string[..]),
            _ => Err(ShellError::type_error("string", self.spanned_type_name())),
        }
    }

    /// View the Value as a path, if possible
    pub fn as_path(&self) -> Result<PathBuf, ShellError> {
        match &self.value {
            UntaggedValue::Primitive(Primitive::FilePath(path)) => Ok(path.clone()),
            UntaggedValue::Primitive(Primitive::String(path_str)) => Ok(PathBuf::from(&path_str)),
            _ => Err(ShellError::type_error("Path", self.spanned_type_name())),
        }
    }

    /// View the Value as a Primitive value, if possible
    pub fn as_primitive(&self) -> Result<Primitive, ShellError> {
        match &self.value {
            UntaggedValue::Primitive(primitive) => Ok(primitive.clone()),
            _ => Err(ShellError::type_error(
                "Primitive",
                self.spanned_type_name(),
            )),
        }
    }

    /// View the Value as a Primitive value, if possible
    pub fn is_primitive(&self) -> bool {
        matches!(&self.value, UntaggedValue::Primitive(_))
    }

    /// View the Value as unsigned 64-bit, if possible
    pub fn as_u64(&self) -> Result<u64, ShellError> {
        match &self.value {
            UntaggedValue::Primitive(primitive) => primitive.as_u64(self.tag.span),
            _ => Err(ShellError::type_error("integer", self.spanned_type_name())),
        }
    }

    /// View the Value as signed 64-bit, if possible
    pub fn as_i64(&self) -> Result<i64, ShellError> {
        match &self.value {
            UntaggedValue::Primitive(primitive) => primitive.as_i64(self.tag.span),
            _ => Err(ShellError::type_error("integer", self.spanned_type_name())),
        }
    }

    /// View the Value as boolean, if possible
    pub fn as_bool(&self) -> Result<bool, ShellError> {
        match &self.value {
            UntaggedValue::Primitive(Primitive::Boolean(p)) => Ok(*p),
            _ => Err(ShellError::type_error("boolean", self.spanned_type_name())),
        }
    }

    /// Returns an iterator of the values rows
    pub fn table_entries(&self) -> TableValueIter<'_> {
        crate::value::iter::table_entries(&self)
    }

    /// Returns an iterator of the value's cells
    pub fn row_entries(&self) -> RowValueIter<'_> {
        crate::value::iter::row_entries(&self)
    }

    /// Returns true if the value is empty
    pub fn is_empty(&self) -> bool {
        match &self {
            Value {
                value: UntaggedValue::Primitive(p),
                ..
            } => p.is_empty(),
            Value {
                value: UntaggedValue::Table(rows),
                ..
            } => rows.is_empty(),
            r
            @
            Value {
                value: UntaggedValue::Row(_),
                ..
            } => r.row_entries().all(|(_, value)| value.is_empty()),
            _ => false,
        }
    }

    pub fn nothing() -> Value {
        UntaggedValue::nothing().into_untagged_value()
    }
}

impl From<String> for Value {
    fn from(s: String) -> Value {
        let end = s.len();
        Value {
            value: s.into(),
            tag: Tag {
                anchor: None,
                span: Span::new(0, end),
            },
        }
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Value {
        let end = s.len();
        Value {
            value: s.into(),
            tag: Tag {
                anchor: None,
                span: Span::new(0, end),
            },
        }
    }
}

impl From<bool> for Value {
    fn from(s: bool) -> Value {
        Value {
            value: s.into(),
            tag: Tag {
                anchor: None,
                span: Span::unknown(),
            },
        }
    }
}

impl<T> From<T> for UntaggedValue
where
    T: Into<Primitive>,
{
    /// Convert a Primitive to an UntaggedValue
    fn from(input: T) -> UntaggedValue {
        UntaggedValue::Primitive(input.into())
    }
}

impl From<ShellError> for UntaggedValue {
    fn from(e: ShellError) -> Self {
        UntaggedValue::Error(e)
    }
}

impl From<UntaggedValue> for Value {
    /// Convert an UntaggedValue into a Value with a default tag
    fn from(value: UntaggedValue) -> Value {
        Value {
            value,
            tag: Tag::default(),
        }
    }
}

impl From<Value> for UntaggedValue {
    /// Convert a Value into an UntaggedValue
    fn from(v: Value) -> UntaggedValue {
        v.value
    }
}

/// Convert a borrowed Value into a borrowed UntaggedValue
impl<'a> From<&'a Value> for &'a UntaggedValue {
    fn from(x: &'a Value) -> Self {
        &x.value
    }
}

impl HasSpan for Value {
    /// Return the corresponding Span for the Value
    fn span(&self) -> Span {
        self.tag.span
    }
}

impl ShellTypeName for Value {
    /// Get the type name for the Value
    fn type_name(&self) -> &'static str {
        ShellTypeName::type_name(&self.value)
    }
}

impl ShellTypeName for UntaggedValue {
    /// Get the type name for the UntaggedValue
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

pub fn merge_descriptors(values: &[Value]) -> Vec<String> {
    let mut ret: Vec<String> = vec![];
    let value_column = "".to_string();
    for value in values {
        let descs = value.data_descriptors();

        if descs.is_empty() {
            if !ret.contains(&value_column) {
                ret.push("".to_string());
            }
        } else {
            for desc in value.data_descriptors() {
                if !ret.contains(&desc) {
                    ret.push(desc);
                }
            }
        }
    }
    ret
}

// Extensions

pub trait StringExt {
    fn to_string_untagged_value(&self) -> UntaggedValue;
    fn to_string_value(&self, tag: Tag) -> Value;
    fn to_string_value_create_tag(&self) -> Value;
    fn to_column_path_value(&self, tag: Tag) -> Value;
    fn to_column_path_untagged_value(&self, span: Span) -> UntaggedValue;
    fn to_pattern_value(&self, tag: Tag) -> Value;
    fn to_pattern_untagged_value(&self) -> UntaggedValue;
}

impl StringExt for String {
    fn to_string_value(&self, the_tag: Tag) -> Value {
        Value {
            value: UntaggedValue::Primitive(Primitive::String(self.to_string())),
            tag: the_tag,
        }
    }

    fn to_string_value_create_tag(&self) -> Value {
        let end = self.len();
        Value {
            value: UntaggedValue::Primitive(Primitive::String(self.to_string())),
            tag: Tag {
                anchor: None,
                span: Span::new(0, end),
            },
        }
    }

    fn to_string_untagged_value(&self) -> UntaggedValue {
        UntaggedValue::string(self)
    }

    fn to_column_path_value(&self, the_tag: Tag) -> Value {
        Value {
            value: UntaggedValue::Primitive(Primitive::ColumnPath(ColumnPath::build(
                &the_tag.span.spanned_string(self), // this is suspect
            ))),
            tag: the_tag,
        }
    }

    fn to_column_path_untagged_value(&self, span: Span) -> UntaggedValue {
        let s = self.to_string().spanned(span);
        UntaggedValue::Primitive(Primitive::ColumnPath(ColumnPath::build(&s)))
    }

    fn to_pattern_value(&self, the_tag: Tag) -> Value {
        Value {
            value: UntaggedValue::Primitive(Primitive::GlobPattern(self.to_string())),
            tag: the_tag,
        }
    }

    fn to_pattern_untagged_value(&self) -> UntaggedValue {
        UntaggedValue::glob_pattern(self)
    }
}

pub trait StrExt {
    fn to_str_untagged_value(&self) -> UntaggedValue;
    fn to_str_value(&self, tag: Tag) -> Value;
    fn to_str_value_create_tag(&self) -> Value;
    fn to_column_path_value(&self, tag: Tag) -> Value;
    fn to_column_path_untagged_value(&self, span: Span) -> UntaggedValue;
    fn to_pattern_value(&self, tag: Tag) -> Value;
    fn to_pattern_untagged_value(&self) -> UntaggedValue;
}

impl StrExt for &str {
    fn to_str_value(&self, the_tag: Tag) -> Value {
        Value {
            value: UntaggedValue::Primitive(Primitive::String(self.to_string())),
            tag: the_tag,
        }
    }

    fn to_str_value_create_tag(&self) -> Value {
        let end = self.len();
        Value {
            value: UntaggedValue::Primitive(Primitive::String(self.to_string())),
            tag: Tag {
                anchor: None,
                span: Span::new(0, end),
            },
        }
    }

    fn to_str_untagged_value(&self) -> UntaggedValue {
        UntaggedValue::string(*self)
    }

    fn to_column_path_value(&self, the_tag: Tag) -> Value {
        Value {
            value: UntaggedValue::Primitive(Primitive::ColumnPath(ColumnPath::build(
                &the_tag.span.spanned_string(self), // this is suspect
            ))),
            tag: the_tag,
        }
    }

    fn to_column_path_untagged_value(&self, span: Span) -> UntaggedValue {
        let s = self.to_string().spanned(span);
        UntaggedValue::Primitive(Primitive::ColumnPath(ColumnPath::build(&s)))
    }

    fn to_pattern_value(&self, the_tag: Tag) -> Value {
        Value {
            value: UntaggedValue::Primitive(Primitive::GlobPattern(self.to_string())),
            tag: the_tag,
        }
    }

    fn to_pattern_untagged_value(&self) -> UntaggedValue {
        UntaggedValue::glob_pattern(*self)
    }
}

pub trait U64Ext {
    fn to_untagged_value(&self) -> UntaggedValue;
    fn to_value(&self, tag: Tag) -> Value;
    fn to_value_create_tag(&self) -> Value;
    fn to_filesize_untagged_value(&self) -> UntaggedValue;
    fn to_filesize_value(&self, tag: Tag) -> Value;
    fn to_duration_untagged_value(&self) -> UntaggedValue;
    fn to_duration_value(&self, tag: Tag) -> Value;
}

impl U64Ext for u64 {
    fn to_value(&self, the_tag: Tag) -> Value {
        Value {
            value: UntaggedValue::Primitive(Primitive::Int(BigInt::from(*self))),
            tag: the_tag,
        }
    }

    fn to_filesize_value(&self, the_tag: Tag) -> Value {
        Value {
            value: UntaggedValue::Primitive(Primitive::Filesize(BigInt::from(*self))),
            tag: the_tag,
        }
    }

    fn to_duration_value(&self, the_tag: Tag) -> Value {
        Value {
            value: UntaggedValue::Primitive(Primitive::Duration(BigInt::from(*self))),
            tag: the_tag,
        }
    }

    fn to_value_create_tag(&self) -> Value {
        let end = self.to_string().len();
        Value {
            value: UntaggedValue::Primitive(Primitive::Int(BigInt::from(*self))),
            tag: Tag {
                anchor: None,
                span: Span::new(0, end),
            },
        }
    }

    fn to_untagged_value(&self) -> UntaggedValue {
        UntaggedValue::int(*self)
    }

    fn to_filesize_untagged_value(&self) -> UntaggedValue {
        UntaggedValue::filesize(*self)
    }

    fn to_duration_untagged_value(&self) -> UntaggedValue {
        UntaggedValue::duration(BigInt::from(*self))
    }
}

pub trait I64Ext {
    fn to_untagged_value(&self) -> UntaggedValue;
    fn to_value(&self, tag: Tag) -> Value;
    fn to_value_create_tag(&self) -> Value;
}

impl I64Ext for i64 {
    fn to_value(&self, the_tag: Tag) -> Value {
        Value {
            value: UntaggedValue::Primitive(Primitive::Int(BigInt::from(*self))),
            tag: the_tag,
        }
    }

    fn to_value_create_tag(&self) -> Value {
        let end = self.to_string().len();
        Value {
            value: UntaggedValue::Primitive(Primitive::Int(BigInt::from(*self))),
            tag: Tag {
                anchor: None,
                span: Span::new(0, end),
            },
        }
    }

    fn to_untagged_value(&self) -> UntaggedValue {
        UntaggedValue::int(*self)
    }
}

pub trait DecimalExt {
    fn to_untagged_value(&self) -> UntaggedValue;
    fn to_value(&self, tag: Tag) -> Value;
    fn to_value_create_tag(&self) -> Value;
}

impl DecimalExt for f64 {
    fn to_value(&self, the_tag: Tag) -> Value {
        if let Some(f) = BigDecimal::from_f64(*self) {
            Value {
                value: UntaggedValue::Primitive(Primitive::Decimal(f)),
                tag: the_tag,
            }
        } else {
            unreachable!("Internal error: protocol did not use f64-compatible decimal")
        }
    }

    fn to_value_create_tag(&self) -> Value {
        let end = self.to_string().len();
        if let Some(f) = BigDecimal::from_f64(*self) {
            Value {
                value: UntaggedValue::Primitive(Primitive::Decimal(f)),
                tag: Tag {
                    anchor: None,
                    span: Span::new(0, end),
                },
            }
        } else {
            unreachable!("Internal error: protocol did not use f64-compatible decimal")
        }
    }

    fn to_untagged_value(&self) -> UntaggedValue {
        if let Some(f) = BigDecimal::from_f64(*self) {
            UntaggedValue::decimal(f)
        } else {
            unreachable!("Internal error: protocol did not use f64-compatible decimal")
        }
    }
}

pub trait PathBufExt {
    fn to_untagged_value(&self) -> UntaggedValue;
    fn to_value(&self, tag: Tag) -> Value;
    fn to_value_create_tag(&self) -> Value;
}

impl PathBufExt for PathBuf {
    fn to_value(&self, the_tag: Tag) -> Value {
        let pb = self.clone();
        Value {
            value: UntaggedValue::Primitive(Primitive::FilePath(pb)),
            tag: the_tag,
        }
    }

    fn to_value_create_tag(&self) -> Value {
        let end = self
            .to_str()
            .expect("unable to convert pathbuf to str")
            .len();
        let pb = self.clone();
        Value {
            value: UntaggedValue::Primitive(Primitive::FilePath(pb)),
            tag: Tag {
                anchor: None,
                span: Span::new(0, end),
            },
        }
    }

    fn to_untagged_value(&self) -> UntaggedValue {
        let pb = self.clone();
        UntaggedValue::filepath(pb)
    }
}

pub trait BooleanExt {
    fn to_untagged_value(&self) -> UntaggedValue;
    fn to_value(&self, tag: Tag) -> Value;
    fn to_value_create_tag(&self) -> Value;
}

impl BooleanExt for bool {
    fn to_value(&self, the_tag: Tag) -> Value {
        Value {
            value: UntaggedValue::Primitive(Primitive::Boolean(*self)),
            tag: the_tag,
        }
    }

    fn to_value_create_tag(&self) -> Value {
        let end = self.to_string().len();
        Value {
            value: UntaggedValue::Primitive(Primitive::Boolean(*self)),
            tag: Tag {
                anchor: None,
                span: Span::new(0, end),
            },
        }
    }

    fn to_untagged_value(&self) -> UntaggedValue {
        UntaggedValue::boolean(*self)
    }
}

pub trait DateTimeExt {
    fn to_untagged_value(&self) -> UntaggedValue;
    fn to_value(&self, tag: Tag) -> Value;
    fn to_value_create_tag(&self) -> Value;
}

impl DateTimeExt for DateTime<FixedOffset> {
    fn to_value(&self, the_tag: Tag) -> Value {
        Value {
            value: UntaggedValue::Primitive(Primitive::Date(*self)),
            tag: the_tag,
        }
    }

    fn to_value_create_tag(&self) -> Value {
        let end = self.to_string().len();
        Value {
            value: UntaggedValue::Primitive(Primitive::Date(*self)),
            tag: Tag {
                anchor: None,
                span: Span::new(0, end),
            },
        }
    }

    fn to_untagged_value(&self) -> UntaggedValue {
        UntaggedValue::date(*self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::indexmap;

    #[test]
    fn test_merge_descriptors() {
        let value = vec![
            UntaggedValue::row(indexmap! {
                "h1".into() => Value::from("Ecuador")
            })
            .into_untagged_value(),
            UntaggedValue::row(indexmap! {
                "h2".into() => Value::from("Ecuador")
            })
            .into_untagged_value(),
            UntaggedValue::row(indexmap! {
                "h3".into() => Value::from("Ecuador")
            })
            .into_untagged_value(),
            UntaggedValue::row(indexmap! {
                "h1".into() => Value::from("Ecuador"),
                "h4".into() => Value::from("Ecuador"),
            })
            .into_untagged_value(),
        ];

        assert_eq!(
            merge_descriptors(&value),
            vec![
                String::from("h1"),
                String::from("h2"),
                String::from("h3"),
                String::from("h4")
            ]
        );
    }

    #[test]
    fn test_data_descriptors() {
        let value = vec![
            UntaggedValue::row(indexmap! {
                "h1".into() => Value::from("Ecuador")
            }),
            UntaggedValue::row(indexmap! {
                "h2".into() => Value::from("Ecuador")
            }),
            UntaggedValue::row(indexmap! {
                "h3".into() => Value::from("Ecuador")
            }),
            UntaggedValue::row(indexmap! {
                "h1".into() => Value::from("Ecuador"),
                "h4".into() => Value::from("Ecuador"),
            }),
        ];

        assert_eq!(
            value
                .iter()
                .map(|v| v.data_descriptors().len())
                .collect::<Vec<_>>(),
            vec![1, 1, 1, 2]
        );
    }

    #[test]
    fn test_decimal_from_float() {
        assert_eq!(
            UntaggedValue::from("inf"),
            UntaggedValue::decimal_from_float(f64::INFINITY, Span::default())
        );
        assert_eq!(
            UntaggedValue::from("-inf"),
            UntaggedValue::decimal_from_float(f64::NEG_INFINITY, Span::default())
        );
        assert_eq!(
            UntaggedValue::from("NaN"),
            UntaggedValue::decimal_from_float(f64::NAN, Span::default())
        );
        assert_eq!(
            UntaggedValue::from(5.5),
            UntaggedValue::decimal_from_float(5.5, Span::default())
        )
    }

    #[test]
    fn test_string_to_string_untagged_value_extension() {
        assert_eq!(
            "a_string".to_string().to_string_untagged_value(),
            UntaggedValue::from("a_string".to_string())
        );
    }

    #[test]
    fn test_string_to_string_value_extension() {
        let end = "a_string".to_string().len();
        let the_tag = Tag {
            anchor: None,
            span: Span::new(0, end),
        };

        let expected = Value {
            value: UntaggedValue::Primitive(Primitive::String("a_string".to_string())),
            tag: the_tag.clone(),
        };

        assert_eq!("a_string".to_string().to_string_value(the_tag), expected);
    }

    #[test]
    fn test_string_to_string_value_create_tag_extension() {
        let end = "a_string".to_string().len();
        let tag = Tag {
            anchor: None,
            span: Span::new(0, end),
        };

        let expected = Value {
            value: UntaggedValue::Primitive(Primitive::String("a_string".to_string())),
            tag,
        };

        assert_eq!(
            "a_string".to_string().to_string_value_create_tag(),
            expected
        );
    }

    #[test]
    fn test_string_to_pattern_untagged_value() {
        let a_pattern = r"[a-zA-Z0-9 ]";
        assert_eq!(
            a_pattern.to_pattern_untagged_value(),
            UntaggedValue::glob_pattern(a_pattern)
        );
    }

    #[test]
    fn test_string_to_column_path_untagged_value() {
        let a_columnpath = "some_column_path";
        let a_span = Span::new(0, a_columnpath.len());
        assert_eq!(
            a_columnpath.to_column_path_untagged_value(a_span),
            UntaggedValue::column_path(a_columnpath, a_span)
        );
    }

    #[test]
    fn test_str_to_str_untaggged_value_extension() {
        assert_eq!(
            "a_str".to_str_untagged_value(),
            UntaggedValue::from("a_str".to_string())
        );
    }
}
