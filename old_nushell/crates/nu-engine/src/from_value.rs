use std::path::PathBuf;

use bigdecimal::{BigDecimal, ToPrimitive};
use chrono::{DateTime, FixedOffset};
use nu_errors::ShellError;
use nu_path::expand_path;
use nu_protocol::{
    hir::CapturedBlock, ColumnPath, Dictionary, Primitive, Range, SpannedTypeName, UntaggedValue,
    Value,
};
use nu_source::{Tagged, TaggedItem};
use num_bigint::BigInt;

pub trait FromValue: Sized {
    fn from_value(v: &Value) -> Result<Self, ShellError>;
}

impl FromValue for Value {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        Ok(v.clone())
    }
}

impl FromValue for Tagged<num_bigint::BigInt> {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        let tag = v.tag.clone();

        match v {
            Value {
                value: UntaggedValue::Primitive(Primitive::Int(i)),
                ..
            } => Ok(BigInt::from(*i).tagged(tag)),
            Value {
                value: UntaggedValue::Primitive(Primitive::Filesize(i)),
                ..
            } => Ok(BigInt::from(*i).tagged(tag)),
            Value {
                value: UntaggedValue::Primitive(Primitive::Duration(i)),
                ..
            } => Ok(i.clone().tagged(tag)),
            Value {
                value: UntaggedValue::Row(_),
                ..
            } => {
                let mut shell_error = ShellError::type_error("integer", v.spanned_type_name());
                shell_error.notes.push(
                    "Note: you can access columns using dot. eg) $it.column or (ls).column".into(),
                );
                Err(shell_error)
            }
            v => Err(ShellError::type_error("integer", v.spanned_type_name())),
        }
    }
}

impl FromValue for num_bigint::BigInt {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        match v {
            Value {
                value: UntaggedValue::Primitive(Primitive::Int(i)),
                ..
            } => Ok(BigInt::from(*i)),
            Value {
                value: UntaggedValue::Primitive(Primitive::Filesize(i)),
                ..
            } => Ok(BigInt::from(*i)),
            Value {
                value: UntaggedValue::Primitive(Primitive::Duration(i)),
                ..
            } => Ok(i.clone()),
            Value {
                value: UntaggedValue::Row(_),
                ..
            } => {
                let mut shell_error = ShellError::type_error("integer", v.spanned_type_name());
                shell_error.notes.push(
                    "Note: you can access columns using dot. eg) $it.column or (ls).column".into(),
                );
                Err(shell_error)
            }
            v => Err(ShellError::type_error("integer", v.spanned_type_name())),
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

impl FromValue for i64 {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        v.as_i64()
    }
}

impl FromValue for Tagged<i64> {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        let tag = v.tag.clone();
        v.as_i64().map(|s| s.tagged(tag))
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
            Value {
                value: UntaggedValue::Primitive(Primitive::Int(i)),
                ..
            } => Ok(BigDecimal::from(*i)),
            Value {
                value: UntaggedValue::Row(_),
                ..
            } => {
                let mut shell_error = ShellError::type_error("decimal", v.spanned_type_name());
                shell_error.notes.push(
                    "Note: you can access columns using dot. eg) $it.column or (ls).column".into(),
                );
                Err(shell_error)
            }
            v => Err(ShellError::type_error("decimal", v.spanned_type_name())),
        }
    }
}

impl FromValue for Tagged<bigdecimal::BigDecimal> {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        let tag = v.tag.clone();
        match &v.value {
            UntaggedValue::Primitive(Primitive::Decimal(d)) => Ok(d.clone().tagged(tag)),
            UntaggedValue::Primitive(Primitive::Int(i)) => Ok(BigDecimal::from(*i).tagged(tag)),
            _ => Err(ShellError::type_error("decimal", v.spanned_type_name())),
        }
    }
}

impl FromValue for Tagged<f64> {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        let tag = v.tag.clone();
        let decimal: bigdecimal::BigDecimal = FromValue::from_value(v)?;

        match decimal.to_f64() {
            Some(d) => Ok(d.tagged(tag)),
            _ => Err(ShellError::type_error("decimal", v.spanned_type_name())),
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
            Value {
                value: UntaggedValue::Row(_),
                ..
            } => {
                let mut shell_error = ShellError::type_error("string", v.spanned_type_name());
                shell_error.notes.push(
                    "Note: you can access columns using dot. eg) $it.column or (ls).column".into(),
                );
                Err(shell_error)
            }
            v => Err(ShellError::type_error("string", v.spanned_type_name())),
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
            } => Ok(expand_path(s)),
            Value {
                value: UntaggedValue::Primitive(Primitive::FilePath(p)),
                ..
            } => Ok(expand_path(p)),
            Value {
                value: UntaggedValue::Row(_),
                ..
            } => {
                let mut shell_error = ShellError::type_error("filepath", v.spanned_type_name());
                shell_error.notes.push(
                    "Note: you can access columns using dot. eg) $it.column or (ls).column".into(),
                );
                Err(shell_error)
            }
            v => Err(ShellError::type_error("filepath", v.spanned_type_name())),
        }
    }
}

impl FromValue for Tagged<PathBuf> {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        match v {
            Value {
                value: UntaggedValue::Primitive(Primitive::String(s)),
                tag,
            } => Ok(expand_path(s).tagged(tag)),
            Value {
                value: UntaggedValue::Primitive(Primitive::FilePath(p)),
                tag,
            } => Ok(expand_path(p).tagged(tag)),
            Value {
                value: UntaggedValue::Row(_),
                ..
            } => {
                let mut shell_error = ShellError::type_error("filepath", v.spanned_type_name());
                shell_error.notes.push(
                    "Note: you can access columns using dot. eg) $it.column or (ls).column".into(),
                );
                Err(shell_error)
            }
            v => Err(ShellError::type_error("filepath", v.spanned_type_name())),
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
            v => Err(ShellError::type_error("column path", v.spanned_type_name())),
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
            Value {
                value: UntaggedValue::Row(_),
                ..
            } => {
                let mut shell_error = ShellError::type_error("boolean", v.spanned_type_name());
                shell_error.notes.push(
                    "Note: you can access columns using dot. eg) $it.column or (ls).column".into(),
                );
                Err(shell_error)
            }
            v => Err(ShellError::type_error("boolean", v.spanned_type_name())),
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
            Value {
                value: UntaggedValue::Row(_),
                ..
            } => {
                let mut shell_error = ShellError::type_error("boolean", v.spanned_type_name());
                shell_error.notes.push(
                    "Note: you can access columns using dot. eg) $it.column or (ls).column".into(),
                );
                Err(shell_error)
            }
            v => Err(ShellError::type_error("boolean", v.spanned_type_name())),
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
            Value {
                value: UntaggedValue::Row(_),
                ..
            } => {
                let mut shell_error = ShellError::type_error("date", v.spanned_type_name());
                shell_error.notes.push(
                    "Note: you can access columns using dot. eg) $it.column or (ls).column".into(),
                );
                Err(shell_error)
            }
            v => Err(ShellError::type_error("date", v.spanned_type_name())),
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
            Value {
                value: UntaggedValue::Row(_),
                ..
            } => {
                let mut shell_error = ShellError::type_error("range", v.spanned_type_name());
                shell_error.notes.push(
                    "Note: you can access columns using dot. eg) $it.column or (ls).column".into(),
                );
                Err(shell_error)
            }
            v => Err(ShellError::type_error("range", v.spanned_type_name())),
        }
    }
}

impl FromValue for Tagged<Range> {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        let tag = v.tag.clone();
        match v {
            Value {
                value: UntaggedValue::Primitive(Primitive::Range(ref range)),
                ..
            } => Ok((*range.clone()).tagged(tag)),
            Value {
                value: UntaggedValue::Row(_),
                ..
            } => {
                let mut shell_error = ShellError::type_error("range", v.spanned_type_name());
                shell_error.notes.push(
                    "Note: you can access columns using dot. eg) $it.column or (ls).column".into(),
                );
                Err(shell_error)
            }
            v => Err(ShellError::type_error("range", v.spanned_type_name())),
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
            Value {
                value: UntaggedValue::Row(_),
                ..
            } => {
                let mut shell_error = ShellError::type_error("binary data", v.spanned_type_name());
                shell_error.notes.push(
                    "Note: you can access columns using dot. eg) $it.column or (ls).column".into(),
                );
                Err(shell_error)
            }
            v => Err(ShellError::type_error("binary data", v.spanned_type_name())),
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
            v => Err(ShellError::type_error("row", v.spanned_type_name())),
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
            Value {
                value: UntaggedValue::Row(_),
                ..
            } => {
                let mut shell_error = ShellError::type_error("block", v.spanned_type_name());
                shell_error.notes.push(
                    "Note: you can access columns using dot. eg) $it.column or (ls).column".into(),
                );
                Err(shell_error)
            }
            v => Err(ShellError::type_error("block", v.spanned_type_name())),
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
            Value {
                value: UntaggedValue::Row(_),
                ..
            } => Ok(vec![v.clone()]),
            v => Err(ShellError::type_error("table", v.spanned_type_name())),
        }
    }
}
