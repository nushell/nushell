pub mod column_path;
mod convert;
mod debug;
pub mod dict;
pub mod evaluate;
pub mod primitive;
pub mod range;
mod serde_bigdecimal;
mod serde_bigint;

use crate::type_name::{ShellTypeName, SpannedTypeName};
use crate::value::dict::Dictionary;
use crate::value::evaluate::Evaluate;
use crate::value::primitive::Primitive;
use crate::value::range::{Range, RangeInclusion};
use crate::{ColumnPath, PathMember};
use bigdecimal::BigDecimal;
use indexmap::IndexMap;
use nu_errors::ShellError;
use nu_source::{AnchorLocation, HasSpan, Span, Spanned, Tag};
use num_bigint::BigInt;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub enum UntaggedValue {
    Primitive(Primitive),
    Row(Dictionary),
    Table(Vec<Value>),

    // Errors are a type of value too
    Error(ShellError),

    Block(Evaluate),
}

impl UntaggedValue {
    pub fn retag(self, tag: impl Into<Tag>) -> Value {
        Value {
            value: self,
            tag: tag.into(),
        }
    }

    pub fn data_descriptors(&self) -> Vec<String> {
        match self {
            UntaggedValue::Primitive(_) => vec![],
            UntaggedValue::Row(columns) => columns.entries.keys().map(|x| x.to_string()).collect(),
            UntaggedValue::Block(_) => vec![],
            UntaggedValue::Table(_) => vec![],
            UntaggedValue::Error(_) => vec![],
        }
    }

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

    pub fn is_true(&self) -> bool {
        match self {
            UntaggedValue::Primitive(Primitive::Boolean(true)) => true,
            _ => false,
        }
    }

    pub fn is_some(&self) -> bool {
        !self.is_none()
    }

    pub fn is_none(&self) -> bool {
        match self {
            UntaggedValue::Primitive(Primitive::Nothing) => true,
            _ => false,
        }
    }

    pub fn is_error(&self) -> bool {
        match self {
            UntaggedValue::Error(_err) => true,
            _ => false,
        }
    }

    pub fn expect_error(&self) -> ShellError {
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

    pub fn table(list: &[Value]) -> UntaggedValue {
        UntaggedValue::Table(list.to_vec())
    }

    pub fn string(s: impl Into<String>) -> UntaggedValue {
        UntaggedValue::Primitive(Primitive::String(s.into()))
    }

    pub fn line(s: impl Into<String>) -> UntaggedValue {
        UntaggedValue::Primitive(Primitive::Line(s.into()))
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

    pub fn range(
        left: (Spanned<Primitive>, RangeInclusion),
        right: (Spanned<Primitive>, RangeInclusion),
    ) -> UntaggedValue {
        UntaggedValue::Primitive(Primitive::Range(Box::new(Range::new(left, right))))
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

    pub fn nothing() -> UntaggedValue {
        UntaggedValue::Primitive(Primitive::Nothing)
    }
}

#[derive(Debug, Clone, PartialOrd, PartialEq, Ord, Eq, Hash, Serialize, Deserialize)]
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

    pub fn as_string(&self) -> Result<String, ShellError> {
        match &self.value {
            UntaggedValue::Primitive(Primitive::String(string)) => Ok(string.clone()),
            UntaggedValue::Primitive(Primitive::Line(line)) => Ok(line.clone() + "\n"),
            _ => Err(ShellError::type_error("string", self.spanned_type_name())),
        }
    }

    pub fn as_forgiving_string(&self) -> Result<&str, ShellError> {
        match &self.value {
            UntaggedValue::Primitive(Primitive::String(string)) => Ok(&string[..]),
            _ => Err(ShellError::type_error("string", self.spanned_type_name())),
        }
    }

    pub fn as_path(&self) -> Result<PathBuf, ShellError> {
        match &self.value {
            UntaggedValue::Primitive(Primitive::Path(path)) => Ok(path.clone()),
            UntaggedValue::Primitive(Primitive::String(path_str)) => Ok(PathBuf::from(&path_str)),
            _ => Err(ShellError::type_error("Path", self.spanned_type_name())),
        }
    }

    pub fn as_primitive(&self) -> Result<Primitive, ShellError> {
        match &self.value {
            UntaggedValue::Primitive(primitive) => Ok(primitive.clone()),
            _ => Err(ShellError::type_error(
                "Primitive",
                self.spanned_type_name(),
            )),
        }
    }

    pub fn as_u64(&self) -> Result<u64, ShellError> {
        match &self.value {
            UntaggedValue::Primitive(primitive) => primitive.as_u64(self.tag.span),
            _ => Err(ShellError::type_error("integer", self.spanned_type_name())),
        }
    }
}

impl Into<UntaggedValue> for &str {
    fn into(self) -> UntaggedValue {
        UntaggedValue::Primitive(Primitive::String(self.to_string()))
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

impl From<Primitive> for UntaggedValue {
    fn from(input: Primitive) -> UntaggedValue {
        UntaggedValue::Primitive(input)
    }
}

impl From<String> for UntaggedValue {
    fn from(input: String) -> UntaggedValue {
        UntaggedValue::Primitive(Primitive::String(input))
    }
}
