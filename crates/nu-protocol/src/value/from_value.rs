use crate::{
    ast::{CellPath, PathMember},
    engine::Closure,
    NuGlob, Range, Record, ShellError, Spanned, Type, Value,
};
use chrono::{DateTime, FixedOffset};
use std::{any, path::PathBuf};

/// A trait for loading a value from a [`Value`].
pub trait FromValue: Sized {
    // TODO: instead of ShellError, maybe we could have a FromValueError that implements Into<ShellError>
    /// Loads a value from a [`Value`].
    ///
    /// Just like [`FromStr`](std::str::FromStr), this operation may fail
    /// because the raw `Value` is able to represent more values than the
    /// expected value here.
    fn from_value(v: Value) -> Result<Self, ShellError>;

    /// Expected `Value` type.
    ///
    /// This is used to print out errors of what type of value is expected for
    /// conversion.
    /// Even if not used in [`from_value`](FromValue::from_value) this should
    /// still be implemented so that other implementations like `Option` or
    /// `Vec` can make use of it.
    /// It is advised to call this method in `from_value` to ensure that
    /// expected type in the error is consistent.
    ///
    /// The derived implementation returns a [`Type::Record`].
    fn expected_type() -> Type {
        Type::Custom(
            any::type_name::<Self>()
                .split(':')
                .last()
                .expect("str::split returns an iterator with at least one element")
                .to_string()
                .into_boxed_str(),
        )
    }
}

impl FromValue for Value {
    fn from_value(v: Value) -> Result<Self, ShellError> {
        Ok(v)
    }

    fn expected_type() -> Type {
        Type::Any
    }
}

impl<T> FromValue for Spanned<T>
where
    T: FromValue,
{
    fn from_value(v: Value) -> Result<Self, ShellError> {
        let span = v.span();
        Ok(Spanned {
            item: T::from_value(v)?,
            span,
        })
    }

    fn expected_type() -> Type {
        T::expected_type()
    }
}

impl<T> FromValue for Vec<T>
where
    T: FromValue,
{
    fn from_value(v: Value) -> Result<Self, ShellError> {
        match v {
            Value::List { vals, .. } => vals
                .into_iter()
                .map(T::from_value)
                .collect::<Result<Vec<T>, ShellError>>(),
            v => Err(ShellError::CantConvert {
                to_type: Self::expected_type().to_string(),
                from_type: v.get_type().to_string(),
                span: v.span(),
                help: None,
            }),
        }
    }

    fn expected_type() -> Type {
        Type::List(Box::new(T::expected_type()))
    }
}

impl<T> FromValue for Option<T>
where
    T: FromValue,
{
    fn from_value(v: Value) -> Result<Self, ShellError> {
        match v {
            Value::Nothing { .. } => Ok(None),
            v => T::from_value(v).map(Option::Some),
        }
    }

    fn expected_type() -> Type {
        T::expected_type()
    }
}

impl FromValue for i64 {
    fn from_value(v: Value) -> Result<Self, ShellError> {
        match v {
            Value::Int { val, .. } => Ok(val),
            Value::Filesize { val, .. } => Ok(val),
            Value::Duration { val, .. } => Ok(val),

            v => Err(ShellError::CantConvert {
                to_type: Self::expected_type().to_string(),
                from_type: v.get_type().to_string(),
                span: v.span(),
                help: None,
            }),
        }
    }

    fn expected_type() -> Type {
        Type::Int
    }
}

impl FromValue for f64 {
    fn from_value(v: Value) -> Result<Self, ShellError> {
        match v {
            Value::Float { val, .. } => Ok(val),
            Value::Int { val, .. } => Ok(val as f64),
            v => Err(ShellError::CantConvert {
                to_type: Self::expected_type().to_string(),
                from_type: v.get_type().to_string(),
                span: v.span(),
                help: None,
            }),
        }
    }

    fn expected_type() -> Type {
        Type::Float
    }
}

impl FromValue for usize {
    fn from_value(v: Value) -> Result<Self, ShellError> {
        let span = v.span();
        match v {
            Value::Int { val, .. } => {
                if val.is_negative() {
                    Err(ShellError::NeedsPositiveValue { span })
                } else {
                    Ok(val as usize)
                }
            }
            Value::Filesize { val, .. } => {
                if val.is_negative() {
                    Err(ShellError::NeedsPositiveValue { span })
                } else {
                    Ok(val as usize)
                }
            }
            Value::Duration { val, .. } => {
                if val.is_negative() {
                    Err(ShellError::NeedsPositiveValue { span })
                } else {
                    Ok(val as usize)
                }
            }

            v => Err(ShellError::CantConvert {
                to_type: Self::expected_type().to_string(),
                from_type: v.get_type().to_string(),
                span: v.span(),
                help: None,
            }),
        }
    }

    fn expected_type() -> Type {
        Type::Custom("non-negative int".to_string().into_boxed_str())
    }
}

impl FromValue for String {
    fn from_value(v: Value) -> Result<Self, ShellError> {
        // FIXME: we may want to fail a little nicer here
        match v {
            Value::CellPath { val, .. } => Ok(val.to_string()),
            Value::String { val, .. } => Ok(val),
            v => Err(ShellError::CantConvert {
                to_type: Self::expected_type().to_string(),
                from_type: v.get_type().to_string(),
                span: v.span(),
                help: None,
            }),
        }
    }

    fn expected_type() -> Type {
        Type::String
    }
}

impl FromValue for NuGlob {
    fn from_value(v: Value) -> Result<Self, ShellError> {
        // FIXME: we may want to fail a little nicer here
        match v {
            Value::CellPath { val, .. } => Ok(NuGlob::Expand(val.to_string())),
            Value::String { val, .. } => Ok(NuGlob::DoNotExpand(val)),
            Value::Glob {
                val,
                no_expand: quoted,
                ..
            } => {
                if quoted {
                    Ok(NuGlob::DoNotExpand(val))
                } else {
                    Ok(NuGlob::Expand(val))
                }
            }
            v => Err(ShellError::CantConvert {
                to_type: Self::expected_type().to_string(),
                from_type: v.get_type().to_string(),
                span: v.span(),
                help: None,
            }),
        }
    }

    fn expected_type() -> Type {
        Type::String
    }
}

impl FromValue for CellPath {
    fn from_value(v: Value) -> Result<Self, ShellError> {
        let span = v.span();
        match v {
            Value::CellPath { val, .. } => Ok(val),
            Value::String { val, .. } => Ok(CellPath {
                members: vec![PathMember::String {
                    val,
                    span,
                    optional: false,
                }],
            }),
            Value::Int { val, .. } => {
                if val.is_negative() {
                    Err(ShellError::NeedsPositiveValue { span })
                } else {
                    Ok(CellPath {
                        members: vec![PathMember::Int {
                            val: val as usize,
                            span,
                            optional: false,
                        }],
                    })
                }
            }
            x => Err(ShellError::CantConvert {
                to_type: Self::expected_type().to_string(),
                from_type: x.get_type().to_string(),
                span,
                help: None,
            }),
        }
    }

    fn expected_type() -> Type {
        Type::CellPath
    }
}

impl FromValue for bool {
    fn from_value(v: Value) -> Result<Self, ShellError> {
        match v {
            Value::Bool { val, .. } => Ok(val),
            v => Err(ShellError::CantConvert {
                to_type: Self::expected_type().to_string(),
                from_type: v.get_type().to_string(),
                span: v.span(),
                help: None,
            }),
        }
    }

    fn expected_type() -> Type {
        Type::Bool
    }
}

impl FromValue for DateTime<FixedOffset> {
    fn from_value(v: Value) -> Result<Self, ShellError> {
        match v {
            Value::Date { val, .. } => Ok(val),
            v => Err(ShellError::CantConvert {
                to_type: Self::expected_type().to_string(),
                from_type: v.get_type().to_string(),
                span: v.span(),
                help: None,
            }),
        }
    }

    fn expected_type() -> Type {
        Type::Date
    }
}

impl FromValue for Range {
    fn from_value(v: Value) -> Result<Self, ShellError> {
        match v {
            Value::Range { val, .. } => Ok(*val),
            v => Err(ShellError::CantConvert {
                to_type: Self::expected_type().to_string(),
                from_type: v.get_type().to_string(),
                span: v.span(),
                help: None,
            }),
        }
    }

    fn expected_type() -> Type {
        Type::Range
    }
}

// this impl is special, as it loads from String or Binary
impl FromValue for Vec<u8> {
    fn from_value(v: Value) -> Result<Self, ShellError> {
        match v {
            Value::Binary { val, .. } => Ok(val),
            Value::String { val, .. } => Ok(val.into_bytes()),
            v => Err(ShellError::CantConvert {
                to_type: Self::expected_type().to_string(),
                from_type: v.get_type().to_string(),
                span: v.span(),
                help: None,
            }),
        }
    }

    fn expected_type() -> Type {
        Type::Binary
    }
}

impl FromValue for PathBuf {
    fn from_value(v: Value) -> Result<Self, ShellError> {
        match v {
            Value::String { val, .. } => Ok(val.into()),
            v => Err(ShellError::CantConvert {
                to_type: Self::expected_type().to_string(),
                from_type: v.get_type().to_string(),
                span: v.span(),
                help: None,
            }),
        }
    }

    fn expected_type() -> Type {
        Type::String
    }
}

impl FromValue for Record {
    fn from_value(v: Value) -> Result<Self, ShellError> {
        match v {
            Value::Record { val, .. } => Ok(val.into_owned()),
            v => Err(ShellError::CantConvert {
                to_type: Self::expected_type().to_string(),
                from_type: v.get_type().to_string(),
                span: v.span(),
                help: None,
            }),
        }
    }

    fn expected_type() -> Type {
        Type::Custom("Record".to_string().into_boxed_str())
    }
}

impl FromValue for Closure {
    fn from_value(v: Value) -> Result<Self, ShellError> {
        match v {
            Value::Closure { val, .. } => Ok(*val),
            v => Err(ShellError::CantConvert {
                to_type: Self::expected_type().to_string(),
                from_type: v.get_type().to_string(),
                span: v.span(),
                help: None,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{engine::Closure, FromValue, Record, Type};

    #[test]
    fn expected_type_default_impl() {
        assert_eq!(
            Record::expected_type(),
            Type::Custom("Record".to_string().into_boxed_str())
        );

        assert_eq!(
            Closure::expected_type(),
            Type::Custom("Closure".to_string().into_boxed_str())
        );
    }
}
