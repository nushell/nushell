use crate::object::base as value;
use crate::parser::hir;
use crate::prelude::*;
use derive_new::new;
use serde_derive::Deserialize;

pub trait Type: std::fmt::Debug + Send {
    type Extractor: ExtractType;

    fn name(&self) -> &'static str;
    fn coerce(&self) -> Option<hir::ExpressionKindHint> {
        None
    }
}

pub trait ExtractType: Sized {
    fn extract(value: &Spanned<Value>) -> Result<Self, ShellError>;
    fn check(value: &'value Spanned<Value>) -> Result<&'value Spanned<Value>, ShellError>;
}

#[derive(Debug, Deserialize, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, new)]
pub struct Any;

impl Type for Any {
    type Extractor = Spanned<Value>;

    fn name(&self) -> &'static str {
        "Any"
    }
}

impl ExtractType for Spanned<Value> {
    fn extract(value: &Spanned<Value>) -> Result<Self, ShellError> {
        Ok(value.clone())
    }

    fn check(value: &'value Spanned<Value>) -> Result<&'value Spanned<Value>, ShellError> {
        Ok(value)
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
    fn extract(value: &Spanned<Value>) -> Result<i64, ShellError> {
        match value {
            &Spanned {
                item: Value::Primitive(Primitive::Int(int)),
                ..
            } => Ok(int),
            other => Err(ShellError::type_error("Integer", other.spanned_type_name())),
        }
    }

    fn check(value: &'value Spanned<Value>) -> Result<&'value Spanned<Value>, ShellError> {
        match value {
            v @ Spanned {
                item: Value::Primitive(Primitive::Int(_)),
                ..
            } => Ok(v),
            other => Err(ShellError::type_error("Integer", other.spanned_type_name())),
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
    fn extract(value: &Spanned<Value>) -> Result<String, ShellError> {
        match value {
            Spanned {
                item: Value::Primitive(Primitive::String(string)),
                ..
            } => Ok(string.clone()),
            other => Err(ShellError::type_error("String", other.spanned_type_name())),
        }
    }

    fn check(value: &'value Spanned<Value>) -> Result<&'value Spanned<Value>, ShellError> {
        match value {
            v @ Spanned {
                item: Value::Primitive(Primitive::String(_)),
                ..
            } => Ok(v),
            other => Err(ShellError::type_error("String", other.spanned_type_name())),
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
    fn check(value: &'value Spanned<Value>) -> Result<&'value Spanned<Value>, ShellError> {
        match value {
            v @ Spanned {
                item: Value::Block(_),
                ..
            } => Ok(v),
            other => Err(ShellError::type_error("Block", other.spanned_type_name())),
        }
    }

    fn extract(value: &Spanned<Value>) -> Result<value::Block, ShellError> {
        match value {
            Spanned {
                item: Value::Block(block),
                ..
            } => Ok(block.clone()),
            other => Err(ShellError::type_error("Block", other.spanned_type_name())),
        }
    }
}
