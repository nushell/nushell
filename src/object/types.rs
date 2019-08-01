use crate::object::base as value;
use crate::parser::hir;
use crate::prelude::*;
use derive_new::new;
use serde_derive::Deserialize;
use std::path::PathBuf;

pub trait Type: std::fmt::Debug + Send {
    type Extractor: ExtractType;

    fn name(&self) -> &'static str;
}

pub trait ExtractType: Sized {
    fn extract(value: &Tagged<Value>) -> Result<Self, ShellError>;
    fn check(value: &'value Tagged<Value>) -> Result<&'value Tagged<Value>, ShellError>;
    fn syntax_type() -> hir::SyntaxType {
        hir::SyntaxType::Any
    }
}

impl<T: ExtractType> ExtractType for Tagged<T> {
    fn extract(value: &Tagged<Value>) -> Result<Tagged<T>, ShellError> {
        Ok(T::extract(value)?.tagged(value.span()))
    }

    fn check(value: &'value Tagged<Value>) -> Result<&'value Tagged<Value>, ShellError> {
        T::check(value)
    }

    fn syntax_type() -> hir::SyntaxType {
        T::syntax_type()
    }
}

#[derive(Debug, Deserialize, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, new)]
pub struct Any;

impl Type for Any {
    type Extractor = Tagged<Value>;

    fn name(&self) -> &'static str {
        "Any"
    }
}

impl ExtractType for Tagged<Value> {
    fn extract(value: &Tagged<Value>) -> Result<Self, ShellError> {
        Ok(value.clone())
    }

    fn check(value: &'value Tagged<Value>) -> Result<&'value Tagged<Value>, ShellError> {
        Ok(value)
    }
}

impl ExtractType for std::path::PathBuf {
    fn syntax_type() -> hir::SyntaxType {
        hir::SyntaxType::Path
    }

    fn extract(value: &'a Tagged<Value>) -> Result<std::path::PathBuf, ShellError> {
        match &value {
            Tagged {
                item: Value::Primitive(Primitive::String(p)),
                ..
            } => Ok(PathBuf::from(p)),
            other => Err(ShellError::type_error("Path", other.tagged_type_name())),
        }
    }

    fn check(value: &'value Tagged<Value>) -> Result<&'value Tagged<Value>, ShellError> {
        match &value {
            v @ Tagged {
                item: Value::Primitive(Primitive::Path(_)),
                ..
            } => Ok(v),
            other => Err(ShellError::type_error("Path", other.tagged_type_name())),
        }
    }
}

#[derive(Debug, Deserialize, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, new)]
pub struct Integer;

impl Type for Integer {
    type Extractor = i64;

    fn name(&self) -> &'static str {
        "Integer"
    }
}

impl ExtractType for i64 {
    fn extract(value: &Tagged<Value>) -> Result<i64, ShellError> {
        match value {
            &Tagged {
                item: Value::Primitive(Primitive::Int(int)),
                ..
            } => Ok(int),
            other => Err(ShellError::type_error("Integer", other.tagged_type_name())),
        }
    }

    fn check(value: &'value Tagged<Value>) -> Result<&'value Tagged<Value>, ShellError> {
        match value {
            v @ Tagged {
                item: Value::Primitive(Primitive::Int(_)),
                ..
            } => Ok(v),
            other => Err(ShellError::type_error("Integer", other.tagged_type_name())),
        }
    }
}

#[derive(Debug, Deserialize, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, new)]
pub struct NuString;

impl Type for NuString {
    type Extractor = String;

    fn name(&self) -> &'static str {
        "Integer"
    }
}

impl ExtractType for String {
    fn extract(value: &Tagged<Value>) -> Result<String, ShellError> {
        match value {
            Tagged {
                item: Value::Primitive(Primitive::String(string)),
                ..
            } => Ok(string.clone()),
            other => Err(ShellError::type_error("String", other.tagged_type_name())),
        }
    }

    fn check(value: &'value Tagged<Value>) -> Result<&'value Tagged<Value>, ShellError> {
        match value {
            v @ Tagged {
                item: Value::Primitive(Primitive::String(_)),
                ..
            } => Ok(v),
            other => Err(ShellError::type_error("String", other.tagged_type_name())),
        }
    }
}

#[derive(Debug, Deserialize, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, new)]
pub struct Block;

impl Type for Block {
    type Extractor = value::Block;

    fn name(&self) -> &'static str {
        "Block"
    }
}

impl ExtractType for value::Block {
    fn check(value: &'value Tagged<Value>) -> Result<&'value Tagged<Value>, ShellError> {
        match value {
            v @ Tagged {
                item: Value::Block(_),
                ..
            } => Ok(v),
            other => Err(ShellError::type_error("Block", other.tagged_type_name())),
        }
    }

    fn extract(value: &Tagged<Value>) -> Result<value::Block, ShellError> {
        match value {
            Tagged {
                item: Value::Block(block),
                ..
            } => Ok(block.clone()),
            other => Err(ShellError::type_error("Block", other.tagged_type_name())),
        }
    }
}
