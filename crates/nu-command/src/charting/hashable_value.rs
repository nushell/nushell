use chrono::{DateTime, FixedOffset};
use nu_protocol::{ShellError, Span, Value};
use std::hash::{Hash, Hasher};

/// A subset of [Value](crate::Value), which is hashable.
/// And it means that we can put the value into something like [HashMap](std::collections::HashMap) or [HashSet](std::collections::HashSet)
/// for further usage like value statistics.
///
/// For now the main way to crate a [HashableValue] is using [from_value](HashableValue::from_value)
#[derive(Eq, Debug)]
pub enum HashableValue {
    Bool(bool),
    Int(i64),
    Float(
        [u8; 8], // because f64 is not hashable, we save it as [u8;8] array to make it hashable.
    ),
    Filesize(i64),
    Duration(i64),
    Date(DateTime<FixedOffset>),
    String(String),
    Binary(Vec<u8>),
}

impl Default for HashableValue {
    fn default() -> Self {
        HashableValue::Bool(false)
    }
}

impl HashableValue {
    /// Try to convert from `value` to self
    ///
    /// A `span` is required because when there is an error in value, it does not contain a `span` field.
    ///
    /// If the given value is not hashable(mainly because of it is structured data), an error will returned.
    pub fn from_value(value: Value, span: Span) -> Result<Self, ShellError> {
        match value {
            Value::Bool(val) => Ok(HashableValue::Bool(val)),
            Value::Int(val) => Ok(HashableValue::Int(val)),
            Value::Filesize(val) => Ok(HashableValue::Filesize(val)),
            Value::Duration(val) => Ok(HashableValue::Duration(val)),
            Value::Date(val) => Ok(HashableValue::Date(val)),
            Value::Float(val) => Ok(HashableValue::Float(val.to_ne_bytes())),
            Value::String(val) => Ok(HashableValue::String(val)),
            Value::Binary(val) => Ok(HashableValue::Binary(val)),

            _ => Err(ShellError::UnsupportedInput(
                format!("input value {value:?} is not hashable"),
                span,
            )),
        }
    }

    /// Convert from self to nu's core data type `Value`.
    pub fn into_value(self) -> Value {
        match self {
            HashableValue::Bool(val) => Value::Bool(val),
            HashableValue::Int(val) => Value::Int(val),
            HashableValue::Filesize(val) => Value::Filesize(val),
            HashableValue::Duration(val) => Value::Duration(val),
            HashableValue::Date(val) => Value::Date(val),
            HashableValue::Float(val) => Value::Float(f64::from_ne_bytes(val)),
            HashableValue::String(val) => Value::String(val),
            HashableValue::Binary(val) => Value::Binary(val),
        }
    }
}

impl Hash for HashableValue {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            HashableValue::Bool(val) => val.hash(state),
            HashableValue::Int(val) => val.hash(state),
            HashableValue::Filesize(val) => val.hash(state),
            HashableValue::Duration(val) => val.hash(state),
            HashableValue::Date(val) => val.hash(state),
            HashableValue::Float(val) => val.hash(state),
            HashableValue::String(val) => val.hash(state),
            HashableValue::Binary(val) => val.hash(state),
        }
    }
}

impl PartialEq for HashableValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (HashableValue::Bool(lhs), HashableValue::Bool(rhs)) => lhs == rhs,
            (HashableValue::Int(lhs), HashableValue::Int(rhs)) => lhs == rhs,
            (HashableValue::Filesize(lhs), HashableValue::Filesize(rhs)) => lhs == rhs,
            (HashableValue::Duration(lhs), HashableValue::Duration(rhs)) => lhs == rhs,
            (HashableValue::Date(lhs), HashableValue::Date(rhs)) => lhs == rhs,
            (HashableValue::Float(lhs), HashableValue::Float(rhs)) => lhs == rhs,
            (HashableValue::String(lhs), HashableValue::String(rhs)) => lhs == rhs,
            (HashableValue::Binary(lhs), HashableValue::Binary(rhs)) => lhs == rhs,
            _ => false,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use nu_protocol::ast::{CellPath, PathMember};
    use std::collections::{HashMap, HashSet};

    #[test]
    fn from_value() {
        let span = Span::test_data();
        let values = vec![
            (Value::Bool(true), HashableValue::Bool(true)),
            (Value::Int(1), HashableValue::Int(1)),
            (Value::Filesize(1), HashableValue::Filesize(1)),
            (Value::Duration(1), HashableValue::Duration(1)),
            (
                Value::Date(
                    DateTime::<FixedOffset>::parse_from_rfc2822("Wed, 18 Feb 2015 23:16:09 GMT")
                        .unwrap(),
                ),
                HashableValue::Date(
                    DateTime::<FixedOffset>::parse_from_rfc2822("Wed, 18 Feb 2015 23:16:09 GMT")
                        .unwrap(),
                ),
            ),
            (
                Value::String("1".to_string()),
                HashableValue::String("1".to_string()),
            ),
            (Value::Binary(vec![1]), HashableValue::Binary(vec![1])),
        ];
        for (val, expect_hashable_val) in values.into_iter() {
            assert_eq!(
                HashableValue::from_value(val, Span { start: 0, end: 0 }).unwrap(),
                expect_hashable_val
            );
        }
    }

    #[test]
    fn from_unhashable_value() {
        let span = Span::test_data();
        let values = [
            Value::List(vec![Value::Bool(true)]),
            Value::Block {
                val: 0,
                captures: HashMap::new(),
            },
            Value::Nothing {},
            Value::Error(ShellError::DidYouMean("what?".to_string(), span)),
            Value::CellPath(CellPath {
                members: vec![PathMember::Int { val: 0, span }],
            }),
        ];
        for v in values {
            assert!(HashableValue::from_value(v, Span { start: 0, end: 0 }).is_err())
        }
    }

    #[test]
    fn from_to_tobe_same() {
        let span = Span::test_data();
        let values = vec![
            Value::Bool(true),
            Value::Int(1),
            Value::Filesize(1),
            Value::Duration(1),
            Value::String("1".to_string()),
            Value::Binary(vec![1]),
        ];
        for val in values.into_iter() {
            let expected_val = val.clone();
            assert_eq!(
                HashableValue::from_value(val, Span { start: 0, end: 0 })
                    .unwrap()
                    .into_value(),
                expected_val
            );
        }
    }

    #[test]
    fn put_to_hashset() {
        let span = Span::test_data();
        let mut set = HashSet::new();
        set.insert(HashableValue::Bool(true));
        assert!(set.contains(&HashableValue::Bool(true,)));
        assert_eq!(set.len(), 1);

        set.insert(HashableValue::Bool(true));
        assert!(set.contains(&HashableValue::Bool(true,)));
        assert_eq!(set.len(), 1);

        set.insert(HashableValue::Int(2));
        assert_eq!(set.len(), 2);
    }
}
