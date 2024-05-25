use crate::{
    ast::{CellPath, PathMember},
    engine::Closure,
    NuGlob, Range, Record, ShellError, Spanned, Type, Value,
};
use chrono::{DateTime, FixedOffset};
use std::{any, cmp::Ordering, path::PathBuf, str::FromStr};

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

// Primitive Types

impl<T, const N: usize> FromValue for [T; N]
where
    T: FromValue,
{
    fn from_value(v: Value) -> Result<Self, ShellError> {
        let span = v.span();
        let v_ty = v.get_type();
        let vec = Vec::<T>::from_value(v)?;
        vec.try_into()
            .map_err(|err_vec: Vec<T>| ShellError::CantConvert {
                to_type: Self::expected_type().to_string(),
                from_type: v_ty.to_string(),
                span,
                help: Some(match err_vec.len().cmp(&N) {
                    Ordering::Less => format!(
                        "input list too short ({}), expected length of {N}, add missing values",
                        err_vec.len()
                    ),
                    Ordering::Equal => {
                        unreachable!("conversion would have worked if the length would be the same")
                    }
                    Ordering::Greater => format!(
                        "input list too long ({}), expected length of {N}, remove trailing values",
                        err_vec.len()
                    ),
                }),
            })
    }

    fn expected_type() -> Type {
        Type::Custom(format!("list<{};{N}>", T::expected_type()).into_boxed_str())
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

impl FromValue for char {
    fn from_value(v: Value) -> Result<Self, ShellError> {
        let span = v.span();
        let v_ty = v.get_type();
        match v {
            Value::String { ref val, .. } => match char::from_str(val) {
                Ok(c) => Ok(c),
                Err(_) => Err(ShellError::CantConvert {
                    to_type: Self::expected_type().to_string(),
                    from_type: v_ty.to_string(),
                    span,
                    help: Some(format!("make the string only one char long")),
                }),
            },
            _ => Err(ShellError::CantConvert {
                to_type: Self::expected_type().to_string(),
                from_type: v_ty.to_string(),
                span,
                help: None,
            }),
        }
    }

    fn expected_type() -> Type {
        Type::String
    }
}

impl FromValue for f32 {
    fn from_value(v: Value) -> Result<Self, ShellError> {
        f64::from_value(v).map(|float| float as f32)
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

macro_rules! impl_from_value_for_int {
    ($type:ty) => {
        impl FromValue for $type {
            fn from_value(v: Value) -> Result<Self, ShellError> {
                let span = v.span();
                let int = i64::from_value(v)?;
                const MIN: i64 = <$type>::MIN as i64;
                const MAX: i64 = <$type>::MAX as i64;
                #[allow(overlapping_range_endpoints)] // calculating MIN-1 is not possible for i64::MIN
                #[allow(unreachable_patterns)] // isize might max out i64 number range
                <$type>::try_from(int).map_err(|_| match int {
                    MIN..=MAX => unreachable!(
                        "int should be within the valid range for {}",
                        stringify!($type)
                    ),
                    i64::MIN..=MIN => ShellError::GenericError {
                        error: "Integer too small".to_string(),
                        msg: format!("{int} is smaller than {}", <$type>::MIN),
                        span: Some(span),
                        help: None,
                        inner: vec![],
                    },
                    MAX..=i64::MAX => ShellError::GenericError {
                        error: "Integer too large".to_string(),
                        msg: format!("{int} is larger than {}", <$type>::MAX),
                        span: Some(span),
                        help: None,
                        inner: vec![],
                    },
                })
            }
        }
    };
}

impl_from_value_for_int!(i8);
impl_from_value_for_int!(i16);
impl_from_value_for_int!(i32);
impl_from_value_for_int!(isize);

macro_rules! impl_from_value_for_uint {
    ($type:ty, $max:expr) => {
        impl FromValue for $type {
            fn from_value(v: Value) -> Result<Self, ShellError> {
                let span = v.span();
                const MAX: i64 = $max;
                match v {
                    Value::Int { val, .. }
                    | Value::Filesize { val, .. }
                    | Value::Duration { val, .. } => {
                        match val {
                            i64::MIN..=-1 => Err(ShellError::NeedsPositiveValue { span }),
                            0..=MAX => Ok(val as $type),
                            #[allow(unreachable_patterns)] // u64 will max out the i64 number range
                            n => Err(ShellError::GenericError {
                                error: "Integer too large".to_string(),
                                msg: format!("{n} is larger than {MAX}"),
                                span: Some(span),
                                help: None,
                                inner: vec![],
                            }),
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
    };
}

// Sadly we cannot implement FromValue for u8 without losing the impl of Vec<u8>,
// Rust would find two possible implementations then, Vec<u8> and Vec<T = u8>,
// and wouldn't compile.
// The blanket implementation for Vec<T> is probably more useful than
// implementing FromValue for u8.

impl_from_value_for_uint!(u16, u16::MAX as i64);
impl_from_value_for_uint!(u32, u32::MAX as i64);
impl_from_value_for_uint!(u64, i64::MAX); // u64::Max would be -1 as i64
#[cfg(target_pointer_width = "64")]
impl_from_value_for_uint!(usize, i64::MAX);
#[cfg(target_pointer_width = "32")]
impl_from_value_for_uint!(usize, usize::MAX);

// Other std Types

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

// This impl is different from Vec<T> as it reads from Value::Binary and
// Value::String instead of Value::List.
// This also denies implementing FromValue for u8.
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

// Blanket std Implementations

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

// Nu Types

impl FromValue for Value {
    fn from_value(v: Value) -> Result<Self, ShellError> {
        Ok(v)
    }

    fn expected_type() -> Type {
        Type::Any
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
}

// Blanket Nu Implementations

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
