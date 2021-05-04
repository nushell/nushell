use std::path::PathBuf;

use chrono::{DateTime, FixedOffset};
use nu_errors::ShellError;
use nu_protocol::{
    hir::CapturedBlock, ColumnPath, Dictionary, Primitive, Range, UntaggedValue, Value,
};
use nu_source::{Tagged, TaggedItem};

pub trait FromValue: Sized {
    fn from_value(v: &Value) -> Result<Self, ShellError>;
}

impl FromValue for Value {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        Ok(v.clone())
    }
}

impl FromValue for num_bigint::BigInt {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        match v {
            Value {
                value: UntaggedValue::Primitive(Primitive::Int(i)),
                ..
            }
            | Value {
                value: UntaggedValue::Primitive(Primitive::Filesize(i)),
                ..
            }
            | Value {
                value: UntaggedValue::Primitive(Primitive::Duration(i)),
                ..
            } => Ok(i.clone()),
            Value { tag, .. } => Err(ShellError::labeled_error(
                "Can't convert to integer",
                "can't convert to integer",
                tag.span,
            )),
        }
    }
}

impl FromValue for Tagged<u64> {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        let tag = v.tag.clone();
        v.as_u64().map(|s| s.tagged(tag))
    }
}

impl FromValue for u64 {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        v.as_u64()
    }
}

impl FromValue for Tagged<u32> {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        let tag = v.tag.clone();
        v.as_u32().map(|s| s.tagged(tag))
    }
}

impl FromValue for Tagged<i16> {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        let tag = v.tag.clone();
        v.as_i16().map(|s| s.tagged(tag))
    }
}

impl FromValue for Tagged<usize> {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        let tag = v.tag.clone();
        v.as_usize().map(|s| s.tagged(tag))
    }
}

impl FromValue for Tagged<char> {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        let tag = v.tag.clone();
        v.as_char().map(|c| c.tagged(tag))
    }
}

impl FromValue for usize {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        v.as_usize()
    }
}

impl FromValue for i64 {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        v.as_i64()
    }
}

impl FromValue for i32 {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        v.as_i32()
    }
}

impl FromValue for bigdecimal::BigDecimal {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        match v {
            Value {
                value: UntaggedValue::Primitive(Primitive::Decimal(d)),
                ..
            } => Ok(d.clone()),
            Value { tag, .. } => Err(ShellError::labeled_error(
                "Can't convert to decimal",
                "can't convert to decimal",
                tag.span,
            )),
        }
    }
}

impl FromValue for String {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        match v {
            Value {
                value: UntaggedValue::Primitive(Primitive::String(s)),
                ..
            } => Ok(s.clone()),
            Value {
                value: UntaggedValue::Primitive(Primitive::GlobPattern(s)),
                ..
            } => Ok(s.clone()),
            Value {
                value: UntaggedValue::Primitive(Primitive::FilePath(p)),
                ..
            } => Ok(p.to_string_lossy().to_string()),
            Value { tag, .. } => Err(ShellError::labeled_error(
                "Can't convert to string",
                "can't convert to string",
                tag.span,
            )),
        }
    }
}

impl FromValue for Tagged<String> {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        let tag = v.tag.clone();
        v.as_string().map(|s| s.tagged(tag))
    }
}

impl FromValue for PathBuf {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        match v {
            Value {
                value: UntaggedValue::Primitive(Primitive::String(s)),
                ..
            } => Ok(PathBuf::from(s)),
            Value {
                value: UntaggedValue::Primitive(Primitive::FilePath(p)),
                ..
            } => Ok(p.clone()),
            Value { tag, .. } => Err(ShellError::labeled_error(
                "Can't convert to filepath",
                "can't convert to filepath",
                tag.span,
            )),
        }
    }
}

impl FromValue for Tagged<PathBuf> {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        match v {
            Value {
                value: UntaggedValue::Primitive(Primitive::String(s)),
                tag,
            } => Ok(PathBuf::from(s).tagged(tag)),
            Value {
                value: UntaggedValue::Primitive(Primitive::FilePath(p)),
                tag,
            } => Ok(p.clone().tagged(tag)),
            Value { tag, .. } => Err(ShellError::labeled_error(
                "Can't convert to filepath",
                "can't convert to filepath",
                tag.span,
            )),
        }
    }
}

impl FromValue for ColumnPath {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        match v {
            Value {
                value: UntaggedValue::Primitive(Primitive::ColumnPath(c)),
                ..
            } => Ok(c.clone()),
            Value { tag, .. } => Err(ShellError::labeled_error(
                "Can't convert to column path",
                "can't convert to column path",
                tag.span,
            )),
        }
    }
}

impl FromValue for bool {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        match v {
            Value {
                value: UntaggedValue::Primitive(Primitive::Boolean(b)),
                ..
            } => Ok(*b),
            Value { tag, .. } => Err(ShellError::labeled_error(
                "Can't convert to boolean",
                "can't convert to boolean",
                tag.span,
            )),
        }
    }
}

impl FromValue for Tagged<bool> {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        match v {
            Value {
                value: UntaggedValue::Primitive(Primitive::Boolean(b)),
                tag,
            } => Ok((*b).tagged(tag)),
            Value { tag, .. } => Err(ShellError::labeled_error(
                "Can't convert to boolean",
                "can't convert to boolean",
                tag.span,
            )),
        }
    }
}

impl FromValue for DateTime<FixedOffset> {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        match v {
            Value {
                value: UntaggedValue::Primitive(Primitive::Date(d)),
                ..
            } => Ok(*d),
            Value { tag, .. } => Err(ShellError::labeled_error(
                "Can't convert to date",
                "can't convert to date",
                tag.span,
            )),
        }
    }
}

impl FromValue for Range {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        match v {
            Value {
                value: UntaggedValue::Primitive(Primitive::Range(r)),
                ..
            } => Ok((**r).clone()),
            Value { tag, .. } => Err(ShellError::labeled_error(
                "Can't convert to range",
                "can't convert to range",
                tag.span,
            )),
        }
    }
}

impl FromValue for Tagged<Range> {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        let tag = v.tag.clone();
        match v.value {
            UntaggedValue::Primitive(Primitive::Range(ref range)) => {
                Ok((*range.clone()).tagged(tag))
            }
            _ => Err(ShellError::labeled_error(
                "Can't convert to range",
                "can't convert to range",
                tag.span,
            )),
        }
    }
}

impl FromValue for Vec<u8> {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        match v {
            Value {
                value: UntaggedValue::Primitive(Primitive::Binary(b)),
                ..
            } => Ok(b.clone()),
            Value {
                value: UntaggedValue::Primitive(Primitive::String(s)),
                ..
            } => Ok(s.bytes().collect()),
            Value { tag, .. } => Err(ShellError::labeled_error(
                "Can't convert to binary data",
                "can't convert to binary data",
                tag.span,
            )),
        }
    }
}

impl FromValue for Dictionary {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        match v {
            Value {
                value: UntaggedValue::Row(r),
                ..
            } => Ok(r.clone()),
            Value { tag, .. } => Err(ShellError::labeled_error(
                "Can't convert to row",
                "can't convert to row",
                tag.span,
            )),
        }
    }
}

impl FromValue for CapturedBlock {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        match v {
            Value {
                value: UntaggedValue::Block(b),
                ..
            } => Ok((**b).clone()),
            Value { tag, .. } => Err(ShellError::labeled_error(
                "Can't convert to block",
                "can't convert to block",
                tag.span,
            )),
        }
    }
}

impl FromValue for Vec<Value> {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        match v {
            Value {
                value: UntaggedValue::Table(t),
                ..
            } => Ok(t.clone()),
            Value { tag, .. } => Err(ShellError::labeled_error(
                "Can't convert to table",
                "can't convert to table",
                tag.span,
            )),
        }
    }
}
