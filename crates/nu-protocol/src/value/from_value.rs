use crate::{
    NuGlob, Range, Record, ShellError, Span, Spanned, Type, Value,
    ast::{CellPath, PathMember},
    casing::Casing,
    engine::Closure,
};
use chrono::{DateTime, FixedOffset};
use std::{
    any,
    cmp::Ordering,
    collections::{HashMap, VecDeque},
    fmt,
    num::{
        NonZeroI8, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroIsize, NonZeroU16, NonZeroU32,
        NonZeroU64, NonZeroUsize,
    },
    path::PathBuf,
    str::FromStr,
};

/// A trait for loading a value from a [`Value`].
///
/// # Derivable
/// This trait can be used with `#[derive]`.
///
/// When derived on structs with named fields, it expects a [`Value::Record`] where each field of
/// the struct maps to a corresponding field in the record.
///
/// - If `#[nu_value(rename = "...")]` is applied to a field, that name will be used as the key in
///   the record.
/// - If `#[nu_value(rename_all = "...")]` is applied on the container (struct) the key of the
///   field will be case-converted accordingly.
/// - If neither attribute is applied, the field name is used as is.
/// - If `#[nu_value(default)]` is applied to a field, the field type's [`Default`] implementation
///   will be used if the corresponding record field is missing
///
/// Supported case conversions include those provided by [`heck`], such as
/// "snake_case", "kebab-case", "PascalCase", and others.
/// Additionally, all values accepted by
/// [`#[serde(rename_all = "...")]`](https://serde.rs/container-attrs.html#rename_all) are valid here.
///
/// For structs with unnamed fields, it expects a [`Value::List`], and the fields are populated in
/// the order they appear in the list.
/// Unit structs expect a [`Value::Nothing`], as they contain no data.
/// Attempting to convert from a non-matching `Value` type will result in an error.
///
/// Only enums with no fields may derive this trait.
/// The expected value representation will be the name of the variant as a [`Value::String`].
///
/// - If `#[nu_value(rename = "...")]` is applied to a variant, that name will be used.
/// - If `#[nu_value(rename_all = "...")]` is applied on the enum container, the name of variant
///   will be case-converted accordingly.
/// - If neither attribute is applied, the variant name will default to
///   ["snake_case"](heck::ToSnakeCase).
///
/// Additionally, you can use `#[nu_value(type_name = "...")]` in the derive macro to set a custom type name
/// for `FromValue::expected_type`. This will result in a `Type::Custom` with the specified type name.
/// This can be useful in situations where the default type name is not desired.
///
/// # Enum Example
/// ```
/// # use nu_protocol::{FromValue, Value, ShellError, record, Span};
/// #
/// # let span = Span::unknown();
/// #
/// #[derive(FromValue, Debug, PartialEq)]
/// #[nu_value(rename_all = "COBOL-CASE", type_name = "birb")]
/// enum Bird {
///     MountainEagle,
///     ForestOwl,
///     #[nu_value(rename = "RIVER-QUACK")]
///     RiverDuck,
/// }
///
/// assert_eq!(
///     Bird::from_value(Value::string("FOREST-OWL", span)).unwrap(),
///     Bird::ForestOwl
/// );
///
/// assert_eq!(
///     Bird::from_value(Value::string("RIVER-QUACK", span)).unwrap(),
///     Bird::RiverDuck
/// );
///
/// assert_eq!(
///     &Bird::expected_type().to_string(),
///     "birb"
/// );
/// ```
///
/// # Struct Example
/// ```
/// # use nu_protocol::{FromValue, Value, ShellError, record, Span};
/// #
/// # let span = Span::unknown();
/// #
/// #[derive(FromValue, PartialEq, Eq, Debug)]
/// #[nu_value(rename_all = "kebab-case")]
/// struct Person {
///     first_name: String,
///     last_name: String,
///     #[nu_value(rename = "age")]
///     age_years: u32,
/// }
///
/// let value = Value::record(record! {
///     "first-name" => Value::string("John", span),
///     "last-name" => Value::string("Doe", span),
///     "age" => Value::int(42, span),
/// }, span);
///
/// assert_eq!(
///     Person::from_value(value).unwrap(),
///     Person {
///         first_name: "John".into(),
///         last_name: "Doe".into(),
///         age_years: 42,
///     }
/// );
/// ```
pub trait FromValue: Sized {
    // TODO: instead of ShellError, maybe we could have a FromValueError that implements Into<ShellError>
    /// Loads a value from a [`Value`].
    ///
    /// This method retrieves a value similarly to how strings are parsed using [`FromStr`].
    /// The operation might fail if the `Value` contains unexpected types or structures.
    fn from_value(v: Value) -> Result<Self, ShellError>;

    /// Expected `Value` type.
    ///
    /// This is used to print out errors of what type of value is expected for conversion.
    /// Even if not used in [`from_value`](FromValue::from_value) this should still be implemented
    /// so that other implementations like `Option` or `Vec` can make use of it.
    /// It is advised to call this method in `from_value` to ensure that expected type in the error
    /// is consistent.
    ///
    /// Unlike the default implementation, derived implementations explicitly reveal the concrete
    /// type, such as [`Type::Record`] or [`Type::List`], instead of an opaque type.
    fn expected_type() -> Type {
        Type::Custom(
            any::type_name::<Self>()
                .split(':')
                .next_back()
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
                    help: Some("make the string only one char long".to_string()),
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

//
// We can not use impl<T: FromValue> FromValue for NonZero<T> as NonZero requires an unstable trait
// As a result, we use this macro to implement FromValue for each NonZero type.
//

macro_rules! impl_from_value_for_nonzero {
    ($nonzero:ty, $base:ty) => {
        impl FromValue for $nonzero {
            fn from_value(v: Value) -> Result<Self, ShellError> {
                let span = v.span();
                let val = <$base>::from_value(v)?;
                <$nonzero>::new(val).ok_or_else(|| ShellError::IncorrectValue {
                    msg: "use a value other than 0".into(),
                    val_span: span,
                    call_span: span,
                })
            }

            fn expected_type() -> Type {
                Type::Int
            }
        }
    };
}

impl_from_value_for_nonzero!(NonZeroU16, u16);
impl_from_value_for_nonzero!(NonZeroU32, u32);
impl_from_value_for_nonzero!(NonZeroU64, u64);
impl_from_value_for_nonzero!(NonZeroUsize, usize);

impl_from_value_for_nonzero!(NonZeroI8, i8);
impl_from_value_for_nonzero!(NonZeroI16, i16);
impl_from_value_for_nonzero!(NonZeroI32, i32);
impl_from_value_for_nonzero!(NonZeroI64, i64);
impl_from_value_for_nonzero!(NonZeroIsize, isize);

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
                    i64::MIN..=MIN => int_too_small_error(int, <$type>::MIN, span),
                    MAX..=i64::MAX => int_too_large_error(int, <$type>::MAX, span),
                })
            }

            fn expected_type() -> Type {
                i64::expected_type()
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
                    Value::Int { val, .. } | Value::Duration { val, .. } => {
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
impl_from_value_for_uint!(usize, usize::MAX as i64);

impl FromValue for () {
    fn from_value(v: Value) -> Result<Self, ShellError> {
        match v {
            Value::Nothing { .. } => Ok(()),
            v => Err(ShellError::CantConvert {
                to_type: Self::expected_type().to_string(),
                from_type: v.get_type().to_string(),
                span: v.span(),
                help: None,
            }),
        }
    }

    fn expected_type() -> Type {
        Type::Nothing
    }
}

macro_rules! tuple_from_value {
    ($template:literal, $($t:ident:$n:tt),+) => {
        impl<$($t),+> FromValue for ($($t,)+) where $($t: FromValue,)+ {
            fn from_value(v: Value) -> Result<Self, ShellError> {
                let span = v.span();
                match v {
                    Value::List { vals, .. } => {
                        let mut deque = VecDeque::from(vals);

                        Ok(($(
                            {
                                let v = deque.pop_front().ok_or_else(|| ShellError::CantFindColumn {
                                    col_name: $n.to_string(),
                                    span: None,
                                    src_span: span
                                })?;
                                $t::from_value(v)?
                            },
                        )*))
                    },
                    v => Err(ShellError::CantConvert {
                        to_type: Self::expected_type().to_string(),
                        from_type: v.get_type().to_string(),
                        span: v.span(),
                        help: None,
                    }),
                }
            }

            fn expected_type() -> Type {
                Type::Custom(
                    format!(
                        $template,
                        $($t::expected_type()),*
                    )
                    .into_boxed_str(),
                )
            }
        }
    };
}

// Tuples in std are implemented for up to 12 elements, so we do it here too.
tuple_from_value!("[{}]", T0:0);
tuple_from_value!("[{}, {}]", T0:0, T1:1);
tuple_from_value!("[{}, {}, {}]", T0:0, T1:1, T2:2);
tuple_from_value!("[{}, {}, {}, {}]", T0:0, T1:1, T2:2, T3:3);
tuple_from_value!("[{}, {}, {}, {}, {}]", T0:0, T1:1, T2:2, T3:3, T4:4);
tuple_from_value!("[{}, {}, {}, {}, {}, {}]", T0:0, T1:1, T2:2, T3:3, T4:4, T5:5);
tuple_from_value!("[{}, {}, {}, {}, {}, {}, {}]", T0:0, T1:1, T2:2, T3:3, T4:4, T5:5, T6:6);
tuple_from_value!("[{}, {}, {}, {}, {}, {}, {}, {}]", T0:0, T1:1, T2:2, T3:3, T4:4, T5:5, T6:6, T7:7);
tuple_from_value!("[{}, {}, {}, {}, {}, {}, {}, {}, {}]", T0:0, T1:1, T2:2, T3:3, T4:4, T5:5, T6:6, T7:7, T8:8);
tuple_from_value!("[{}, {}, {}, {}, {}, {}, {}, {}, {}, {}]", T0:0, T1:1, T2:2, T3:3, T4:4, T5:5, T6:6, T7:7, T8:8, T9:9);
tuple_from_value!("[{}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}]", T0:0, T1:1, T2:2, T3:3, T4:4, T5:5, T6:6, T7:7, T8:8, T9:9, T10:10);
tuple_from_value!("[{}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}]", T0:0, T1:1, T2:2, T3:3, T4:4, T5:5, T6:6, T7:7, T8:8, T9:9, T10:10, T11:11);

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

// This impl is different from Vec<T> as it allows reading from Value::Binary and Value::String too.
// This also denies implementing FromValue for u8 as it would be in conflict with the Vec<T> impl.
impl FromValue for Vec<u8> {
    fn from_value(v: Value) -> Result<Self, ShellError> {
        match v {
            Value::Binary { val, .. } => Ok(val),
            Value::String { val, .. } => Ok(val.into_bytes()),
            Value::List { vals, .. } => {
                const U8MIN: i64 = u8::MIN as i64;
                const U8MAX: i64 = u8::MAX as i64;
                let mut this = Vec::with_capacity(vals.len());
                for val in vals {
                    let span = val.span();
                    let int = i64::from_value(val)?;
                    // calculating -1 on these ranges would be less readable
                    #[allow(overlapping_range_endpoints)]
                    #[allow(clippy::match_overlapping_arm)]
                    match int {
                        U8MIN..=U8MAX => this.push(int as u8),
                        i64::MIN..=U8MIN => return Err(int_too_small_error(int, U8MIN, span)),
                        U8MAX..=i64::MAX => return Err(int_too_large_error(int, U8MAX, span)),
                    };
                }
                Ok(this)
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

impl<V> FromValue for HashMap<String, V>
where
    V: FromValue,
{
    fn from_value(v: Value) -> Result<Self, ShellError> {
        let record = v.into_record()?;
        let items: Result<Vec<(String, V)>, ShellError> = record
            .into_iter()
            .map(|(k, v)| Ok((k, V::from_value(v)?)))
            .collect();
        Ok(HashMap::from_iter(items?))
    }

    fn expected_type() -> Type {
        Type::Record(vec![].into_boxed_slice())
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
                .map(|v| T::from_value(v))
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
                    casing: Casing::Sensitive,
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

// Foreign Types

impl FromValue for bytes::Bytes {
    fn from_value(v: Value) -> Result<Self, ShellError> {
        match v {
            Value::Binary { val, .. } => Ok(val.into()),
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

// Use generics with `fmt::Display` to allow passing different kinds of integer
fn int_too_small_error(int: impl fmt::Display, min: impl fmt::Display, span: Span) -> ShellError {
    ShellError::GenericError {
        error: "Integer too small".to_string(),
        msg: format!("{int} is smaller than {min}"),
        span: Some(span),
        help: None,
        inner: vec![],
    }
}

fn int_too_large_error(int: impl fmt::Display, max: impl fmt::Display, span: Span) -> ShellError {
    ShellError::GenericError {
        error: "Integer too large".to_string(),
        msg: format!("{int} is larger than {max}"),
        span: Some(span),
        help: None,
        inner: vec![],
    }
}

#[cfg(test)]
mod tests {
    use crate::{FromValue, IntoValue, Record, Span, Type, Value, engine::Closure};
    use std::ops::Deref;

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

    #[test]
    fn from_value_vec_u8() {
        let vec: Vec<u8> = vec![1, 2, 3];
        let span = Span::test_data();
        let string = "Hello Vec<u8>!".to_string();

        assert_eq!(
            Vec::<u8>::from_value(vec.clone().into_value(span)).unwrap(),
            vec.clone(),
            "Vec<u8> roundtrip"
        );

        assert_eq!(
            Vec::<u8>::from_value(Value::test_string(string.clone()))
                .unwrap()
                .deref(),
            string.as_bytes(),
            "Vec<u8> from String"
        );

        assert_eq!(
            Vec::<u8>::from_value(Value::test_binary(vec.clone())).unwrap(),
            vec,
            "Vec<u8> from Binary"
        );

        assert!(Vec::<u8>::from_value(vec![u8::MIN as i32 - 1].into_value(span)).is_err());
        assert!(Vec::<u8>::from_value(vec![u8::MAX as i32 + 1].into_value(span)).is_err());
    }
}
