use chrono::{DateTime, FixedOffset};
use nu_protocol::{Filesize, ShellError, Span, Value};
use std::hash::{Hash, Hasher};

/// A subset of [`Value`], which is hashable.
/// And it means that we can put the value into something like
/// [`HashMap`](std::collections::HashMap) or [`HashSet`](std::collections::HashSet) for further
/// usage like value statistics.
///
/// For now the main way to crate a [`HashableValue`] is using
/// [`from_value`](HashableValue::from_value)
///
/// Please note that although each variant contains `span` field, but during hashing, this field will not be concerned.
/// Which means that the following will be true:
/// ```text
/// assert_eq!(HashableValue::Bool {val: true, span: Span{start: 0, end: 1}}, HashableValue::Bool {val: true, span: Span{start: 90, end: 1000}})
/// ```
#[derive(Eq, Debug, Ord, PartialOrd)]
pub enum HashableValue {
    Bool {
        val: bool,
        span: Span,
    },
    Int {
        val: i64,
        span: Span,
    },
    Float {
        val: [u8; 8], // because f64 is not hashable, we save it as [u8;8] array to make it hashable.
        span: Span,
    },
    Filesize {
        val: Filesize,
        span: Span,
    },
    Duration {
        val: i64,
        span: Span,
    },
    Date {
        val: DateTime<FixedOffset>,
        span: Span,
    },
    String {
        val: String,
        span: Span,
    },
    Binary {
        val: Vec<u8>,
        span: Span,
    },
}

impl Default for HashableValue {
    fn default() -> Self {
        HashableValue::Bool {
            val: false,
            span: Span::unknown(),
        }
    }
}

impl HashableValue {
    /// Try to convert from `value` to self
    ///
    /// A `span` is required because when there is an error in value, it may not contain `span` field.
    ///
    /// If the given value is not hashable(mainly because of it is structured data), an error will returned.
    pub fn from_value(value: Value, span: Span) -> Result<Self, ShellError> {
        let val_span = value.span();
        match value {
            Value::Bool { val, .. } => Ok(HashableValue::Bool {
                val,
                span: val_span,
            }),
            Value::Int { val, .. } => Ok(HashableValue::Int {
                val,
                span: val_span,
            }),
            Value::Filesize { val, .. } => Ok(HashableValue::Filesize {
                val,
                span: val_span,
            }),
            Value::Duration { val, .. } => Ok(HashableValue::Duration {
                val,
                span: val_span,
            }),
            Value::Date { val, .. } => Ok(HashableValue::Date {
                val,
                span: val_span,
            }),
            Value::Float { val, .. } => Ok(HashableValue::Float {
                val: val.to_ne_bytes(),
                span: val_span,
            }),
            Value::String { val, .. } => Ok(HashableValue::String {
                val,
                span: val_span,
            }),
            Value::Binary { val, .. } => Ok(HashableValue::Binary {
                val,
                span: val_span,
            }),

            // Explicitly propagate errors instead of dropping them.
            Value::Error { error, .. } => Err(*error),
            _ => Err(ShellError::UnsupportedInput {
                msg: "input value is not hashable".into(),
                input: format!("input type: {:?}", value.get_type()),
                msg_span: span,
                input_span: value.span(),
            }),
        }
    }

    /// Convert from self to nu's core data type `Value`.
    pub fn into_value(self) -> Value {
        match self {
            HashableValue::Bool { val, span } => Value::bool(val, span),
            HashableValue::Int { val, span } => Value::int(val, span),
            HashableValue::Filesize { val, span } => Value::filesize(val, span),
            HashableValue::Duration { val, span } => Value::duration(val, span),
            HashableValue::Date { val, span } => Value::date(val, span),
            HashableValue::Float { val, span } => Value::float(f64::from_ne_bytes(val), span),
            HashableValue::String { val, span } => Value::string(val, span),
            HashableValue::Binary { val, span } => Value::binary(val, span),
        }
    }
}

impl Hash for HashableValue {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            HashableValue::Bool { val, .. } => val.hash(state),
            HashableValue::Int { val, .. } => val.hash(state),
            HashableValue::Filesize { val, .. } => val.hash(state),
            HashableValue::Duration { val, .. } => val.hash(state),
            HashableValue::Date { val, .. } => val.hash(state),
            HashableValue::Float { val, .. } => val.hash(state),
            HashableValue::String { val, .. } => val.hash(state),
            HashableValue::Binary { val, .. } => val.hash(state),
        }
    }
}

impl PartialEq for HashableValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (HashableValue::Bool { val: lhs, .. }, HashableValue::Bool { val: rhs, .. }) => {
                lhs == rhs
            }
            (HashableValue::Int { val: lhs, .. }, HashableValue::Int { val: rhs, .. }) => {
                lhs == rhs
            }
            (
                HashableValue::Filesize { val: lhs, .. },
                HashableValue::Filesize { val: rhs, .. },
            ) => lhs == rhs,
            (
                HashableValue::Duration { val: lhs, .. },
                HashableValue::Duration { val: rhs, .. },
            ) => lhs == rhs,
            (HashableValue::Date { val: lhs, .. }, HashableValue::Date { val: rhs, .. }) => {
                lhs == rhs
            }
            (HashableValue::Float { val: lhs, .. }, HashableValue::Float { val: rhs, .. }) => {
                lhs == rhs
            }
            (HashableValue::String { val: lhs, .. }, HashableValue::String { val: rhs, .. }) => {
                lhs == rhs
            }
            (HashableValue::Binary { val: lhs, .. }, HashableValue::Binary { val: rhs, .. }) => {
                lhs == rhs
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use nu_protocol::{
        ast::{CellPath, PathMember},
        engine::Closure,
        BlockId,
    };
    use std::collections::HashSet;

    #[test]
    fn from_value() {
        let span = Span::test_data();
        let values = vec![
            (
                Value::bool(true, span),
                HashableValue::Bool { val: true, span },
            ),
            (Value::int(1, span), HashableValue::Int { val: 1, span }),
            (
                Value::filesize(1, span),
                HashableValue::Filesize {
                    val: 1.into(),
                    span,
                },
            ),
            (
                Value::duration(1, span),
                HashableValue::Duration { val: 1, span },
            ),
            (
                Value::date(
                    DateTime::<FixedOffset>::parse_from_rfc2822("Wed, 18 Feb 2015 23:16:09 GMT")
                        .unwrap(),
                    span,
                ),
                HashableValue::Date {
                    val: DateTime::<FixedOffset>::parse_from_rfc2822(
                        "Wed, 18 Feb 2015 23:16:09 GMT",
                    )
                    .unwrap(),
                    span,
                },
            ),
            (
                Value::string("1".to_string(), span),
                HashableValue::String {
                    val: "1".to_string(),
                    span,
                },
            ),
            (
                Value::binary(vec![1], span),
                HashableValue::Binary { val: vec![1], span },
            ),
        ];
        for (val, expect_hashable_val) in values.into_iter() {
            assert_eq!(
                HashableValue::from_value(val, Span::unknown()).unwrap(),
                expect_hashable_val
            );
        }
    }

    #[test]
    fn from_unhashable_value() {
        let span = Span::test_data();
        let values = [
            Value::list(vec![Value::bool(true, span)], span),
            Value::closure(
                Closure {
                    block_id: BlockId::new(0),
                    captures: Vec::new(),
                },
                span,
            ),
            Value::nothing(span),
            Value::error(
                ShellError::DidYouMean {
                    suggestion: "what?".to_string(),
                    span,
                },
                span,
            ),
            Value::cell_path(
                CellPath {
                    members: vec![PathMember::Int {
                        val: 0,
                        span,
                        optional: false,
                    }],
                },
                span,
            ),
        ];
        for v in values {
            assert!(HashableValue::from_value(v, Span::unknown()).is_err())
        }
    }

    #[test]
    fn from_to_tobe_same() {
        let span = Span::test_data();
        let values = vec![
            Value::bool(true, span),
            Value::int(1, span),
            Value::filesize(1, span),
            Value::duration(1, span),
            Value::string("1".to_string(), span),
            Value::binary(vec![1], span),
        ];
        for val in values.into_iter() {
            let expected_val = val.clone();
            assert_eq!(
                HashableValue::from_value(val, Span::unknown())
                    .unwrap()
                    .into_value(),
                expected_val
            );
        }
    }

    #[test]
    fn hashable_value_eq_without_concern_span() {
        assert_eq!(
            HashableValue::Bool {
                val: true,
                span: Span::new(0, 1)
            },
            HashableValue::Bool {
                val: true,
                span: Span::new(90, 1000)
            }
        )
    }

    #[test]
    fn put_to_hashset() {
        let span = Span::test_data();
        let mut set = HashSet::new();
        set.insert(HashableValue::Bool { val: true, span });
        assert!(set.contains(&HashableValue::Bool { val: true, span }));

        // hashable value doesn't care about span.
        let diff_span = Span::new(1, 2);
        set.insert(HashableValue::Bool {
            val: true,
            span: diff_span,
        });
        assert!(set.contains(&HashableValue::Bool { val: true, span }));
        assert!(set.contains(&HashableValue::Bool {
            val: true,
            span: diff_span
        }));
        assert_eq!(set.len(), 1);

        set.insert(HashableValue::Int { val: 2, span });
        assert_eq!(set.len(), 2);
    }
}
