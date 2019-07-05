use crate::object::base as value;
use crate::parser::hir;
use crate::prelude::*;
use derive_new::new;
use serde_derive::{Deserialize, Serialize};

pub trait Type: std::fmt::Debug + Send {
    fn name(&self) -> &'static str;
    fn check(&self, value: Spanned<Value>) -> Result<Spanned<Value>, ShellError>;
    fn coerce(&self) -> Option<hir::ExpressionKindHint> {
        None
    }
}

pub trait ExtractType<T>: Type {
    fn extract(value: Value) -> T;
}

#[derive(Debug, Deserialize, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, new)]
pub struct Any;

impl Type for Any {
    fn name(&self) -> &'static str {
        "Any"
    }

    fn check(&self, value: Spanned<Value>) -> Result<Spanned<Value>, ShellError> {
        Ok(value)
    }
}

#[derive(Debug, Deserialize, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, new)]
pub struct Integer;

impl Type for Integer {
    fn name(&self) -> &'static str {
        "Integer"
    }

    fn check(&self, value: Spanned<Value>) -> Result<Spanned<Value>, ShellError> {
        match value {
            v @ Spanned {
                item: Value::Primitive(Primitive::Int(_)),
                ..
            } => Ok(v),
            other => Err(ShellError::type_error("Integer", other.spanned_type_name())),
        }
    }
}

impl ExtractType<i64> for Integer {
    fn extract(value: Value) -> i64 {
        match value {
            Value::Primitive(Primitive::Int(int)) => int,
            _ => unreachable!("invariant: must check before extract"),
        }
    }
}

#[derive(Debug, Deserialize, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, new)]
pub struct Block;

impl Type for Block {
    fn name(&self) -> &'static str {
        "Block"
    }

    fn check(&self, value: Spanned<Value>) -> Result<Spanned<Value>, ShellError> {
        match value {
            v @ Spanned {
                item: Value::Block(_),
                ..
            } => Ok(v),
            other => Err(ShellError::type_error("Block", other.spanned_type_name())),
        }
    }
}

impl ExtractType<value::Block> for Block {
    fn extract(value: Value) -> value::Block {
        match value {
            Value::Block(block) => block,
            _ => unreachable!("invariant: must check before extract"),
        }
    }
}
