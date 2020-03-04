use crate::prelude::*;
use log::trace;
use nu_errors::{CoerceInto, ShellError};
use nu_protocol::{Primitive, SpannedTypeName, UntaggedValue, Value};
use nu_source::Tagged;

pub trait ExtractType: Sized {
    fn extract(value: &Value) -> Result<Self, ShellError>;
}

impl<T: ExtractType> ExtractType for Tagged<T> {
    fn extract(value: &Value) -> Result<Tagged<T>, ShellError> {
        let name = std::any::type_name::<T>();
        trace!("<Tagged> Extracting {:?} for Tagged<{}>", value, name);

        Ok(T::extract(value)?.tagged(value.tag()))
    }
}

impl ExtractType for bool {
    fn extract(value: &Value) -> Result<bool, ShellError> {
        trace!("Extracting {:?} for bool", value);

        match &value {
            Value {
                value: UntaggedValue::Primitive(Primitive::Boolean(b)),
                ..
            } => Ok(*b),
            Value {
                value: UntaggedValue::Primitive(Primitive::Nothing),
                ..
            } => Ok(false),
            other => Err(ShellError::type_error("Boolean", other.spanned_type_name())),
        }
    }
}

impl ExtractType for std::path::PathBuf {
    fn extract(value: &Value) -> Result<std::path::PathBuf, ShellError> {
        trace!("Extracting {:?} for PathBuf", value);

        match &value {
            Value {
                value: UntaggedValue::Primitive(Primitive::Path(p)),
                ..
            } => Ok(p.clone()),
            other => Err(ShellError::type_error("Path", other.spanned_type_name())),
        }
    }
}

impl ExtractType for i64 {
    fn extract(value: &Value) -> Result<i64, ShellError> {
        trace!("Extracting {:?} for i64", value);

        match &value {
            &Value {
                value: UntaggedValue::Primitive(Primitive::Int(int)),
                ..
            } => Ok(int.tagged(&value.tag).coerce_into("converting to i64")?),
            other => Err(ShellError::type_error("Integer", other.spanned_type_name())),
        }
    }
}

impl ExtractType for u64 {
    fn extract(value: &Value) -> Result<u64, ShellError> {
        trace!("Extracting {:?} for u64", value);

        match &value {
            &Value {
                value: UntaggedValue::Primitive(Primitive::Int(int)),
                ..
            } => Ok(int.tagged(&value.tag).coerce_into("converting to u64")?),
            other => Err(ShellError::type_error("Integer", other.spanned_type_name())),
        }
    }
}

impl ExtractType for String {
    fn extract(value: &Value) -> Result<String, ShellError> {
        trace!("Extracting {:?} for String", value);

        match value {
            Value {
                value: UntaggedValue::Primitive(Primitive::String(string)),
                ..
            } => Ok(string.clone()),
            Value {
                value: UntaggedValue::Primitive(Primitive::Line(string)),
                ..
            } => Ok(string.clone()),
            other => Err(ShellError::type_error("String", other.spanned_type_name())),
        }
    }
}
