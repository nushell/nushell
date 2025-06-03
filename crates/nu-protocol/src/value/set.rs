use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::hash::Hash;

use crate::{ShellError, Value};
use crate::{Span, SyntaxShape, Type};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CustomSet {
    vals: HashSet<SetValue>,
}

impl CustomSet {
    pub fn new(vals: Vec<Value>) -> Result<Self, ShellError> {
        Ok(Self {
            vals: vals
                .iter()
                .filter_map(|v| SetValue::from_value(v).ok())
                .collect(),
        })
    }

    // TODO : correct error ?
    pub fn push(&mut self, val: &Value) -> Result<(), ShellError> {
        self.vals.insert(SetValue::from_value(val)?);
        Ok(())
    }
}

impl CustomSet {
    pub fn len(&self) -> usize {
        self.vals.len()
    }
    pub fn is_empty(&self) -> bool {
        self.vals.is_empty()
    }
}

impl IntoIterator for CustomSet {
    type Item = SetValue;
    type IntoIter = std::collections::hash_set::IntoIter<SetValue>;

    fn into_iter(self) -> Self::IntoIter {
        self.vals.into_iter()
    }
}

impl<'a> IntoIterator for &'a CustomSet {
    type Item = &'a SetValue;
    type IntoIter = std::collections::hash_set::Iter<'a, SetValue>;

    fn into_iter(self) -> Self::IntoIter {
        self.vals.iter()
    }
}

impl PartialOrd for CustomSet {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.vals.len().partial_cmp(&other.vals.len())
    }
}

impl PartialEq for CustomSet {
    fn eq(&self, other: &Self) -> bool {
        self.vals == other.vals
    }
}

impl Eq for CustomSet {}

// TODO:  add more
#[derive(Debug, Serialize, Deserialize, Hash, Clone, PartialEq, PartialOrd, Eq)]
pub enum SetValue {
    Int(i64),
}

impl SetValue {
    pub fn is_subtype_of(&self, other: &SetType) -> bool {
        SetType::from_value(self)
            .to_type()
            .is_subtype_of(&other.to_type())
    }

    pub fn to_value(&self) -> Value {
        match self {
            SetValue::Int(val) => Value::int(*val, Span::unknown()), // TODO : is unknown the best ?
        }
    }

    pub fn from_value(value: &Value) -> Result<Self, ShellError> {
        match value {
            Value::Int { val, .. } => Ok(SetValue::Int(*val)),
            Value::Bool { .. }
            | Value::Float { .. }
            | Value::String { .. }
            | Value::Glob { .. }
            | Value::Filesize { .. }
            | Value::Duration { .. }
            | Value::Date { .. }
            | Value::Range { .. }
            | Value::Record { .. }
            | Value::Set { .. }
            | Value::List { .. }
            | Value::Closure { .. }
            | Value::Error { .. }
            | Value::Binary { .. }
            | Value::CellPath { .. }
            | Value::Custom { .. }
            | Value::Nothing { .. } => Err(ShellError::TypeMismatch {
                err_message: String::from("Set does not access this type"),
                span: todo!(),
            }),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum SetType {
    Int,
    Any,
    #[default]
    Nothing,
}

// TODO : sure about any ?
impl SetType {
    pub fn to_type(&self) -> Type {
        match self {
            SetType::Int => Type::Int,
            SetType::Any => Type::Any,
            SetType::Nothing => Type::Nothing,
        }
    }

    pub fn from_type(ty: Type) -> Self {
        match ty {
            Type::Int => SetType::Int,
            Type::Any => SetType::Any,
            _ => SetType::Nothing,
        }
    }

    pub fn from_value(value: &SetValue) -> Self {
        match value {
            SetValue::Int(_) => SetType::Int,
        }
    }

    pub fn to_shape(&self) -> SyntaxShape {
        match self {
            SetType::Int => todo!(),
            SetType::Any => todo!(),
            SetType::Nothing => todo!(),
        }
    }
}

impl std::fmt::Display for SetType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SetType::Int => write!(f, "Int"),
            SetType::Any => write!(f, "Any"),
            SetType::Nothing => write!(f, "Nothing"),
        }
    }
}
