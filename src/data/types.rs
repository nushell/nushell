use crate::prelude::*;
use log::trace;

pub trait ExtractType: Sized {
    fn extract(value: &Tagged<Value>) -> Result<Self, ShellError>;
}

impl<T: ExtractType> ExtractType for Tagged<T> {
    fn extract(value: &Tagged<Value>) -> Result<Tagged<T>, ShellError> {
        let name = std::any::type_name::<T>();
        trace!("<Tagged> Extracting {:?} for Tagged<{}>", value, name);

        Ok(T::extract(value)?.tagged(value.tag()))
    }
}

impl ExtractType for bool {
    fn extract(value: &Tagged<Value>) -> Result<bool, ShellError> {
        trace!("Extracting {:?} for bool", value);

        match &value {
            Tagged {
                item: Value::Primitive(Primitive::Boolean(b)),
                ..
            } => Ok(*b),
            Tagged {
                item: Value::Primitive(Primitive::Nothing),
                ..
            } => Ok(false),
            other => Err(ShellError::type_error(
                "Boolean",
                other.type_name().spanned(other.span()),
            )),
        }
    }
}

impl ExtractType for std::path::PathBuf {
    fn extract(value: &Tagged<Value>) -> Result<std::path::PathBuf, ShellError> {
        trace!("Extracting {:?} for PathBuf", value);

        match &value {
            Tagged {
                item: Value::Primitive(Primitive::Path(p)),
                ..
            } => Ok(p.clone()),
            other => Err(ShellError::type_error(
                "Path",
                other.type_name().spanned(other.span()),
            )),
        }
    }
}

impl ExtractType for i64 {
    fn extract(value: &Tagged<Value>) -> Result<i64, ShellError> {
        trace!("Extracting {:?} for i64", value);

        match &value {
            &Tagged {
                item: Value::Primitive(Primitive::Int(int)),
                ..
            } => Ok(int.tagged(&value.tag).coerce_into("converting to i64")?),
            other => Err(ShellError::type_error(
                "Integer",
                other.type_name().spanned(other.span()),
            )),
        }
    }
}

impl ExtractType for u64 {
    fn extract(value: &Tagged<Value>) -> Result<u64, ShellError> {
        trace!("Extracting {:?} for u64", value);

        match &value {
            &Tagged {
                item: Value::Primitive(Primitive::Int(int)),
                ..
            } => Ok(int.tagged(&value.tag).coerce_into("converting to u64")?),
            other => Err(ShellError::type_error(
                "Integer",
                other.type_name().spanned(other.span()),
            )),
        }
    }
}

impl ExtractType for String {
    fn extract(value: &Tagged<Value>) -> Result<String, ShellError> {
        trace!("Extracting {:?} for String", value);

        match value {
            Tagged {
                item: Value::Primitive(Primitive::String(string)),
                ..
            } => Ok(string.clone()),
            other => Err(ShellError::type_error(
                "String",
                other.type_name().spanned(other.span()),
            )),
        }
    }
}
