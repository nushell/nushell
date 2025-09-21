use crate::{Range, Record, ShellError, Span, Value, ast::CellPath, engine::Closure};
use chrono::{DateTime, FixedOffset};
use std::{
    borrow::{Borrow, Cow},
    collections::HashMap,
};

/// A trait for converting a value into a [`Value`].
///
/// This conversion is infallible, for fallible conversions use [`TryIntoValue`].
///
/// # Derivable
/// This trait can be used with `#[derive]`.
/// When derived on structs with named fields, the resulting value representation will use
/// [`Value::Record`], where each field of the record corresponds to a field of the struct.
///
/// By default, field names will be used as-is unless specified otherwise:
/// - If `#[nu_value(rename = "...")]` is applied to a specific field, that name is used.
/// - If `#[nu_value(rename_all = "...")]` is applied to the struct, field names will be
///   case-converted accordingly.
/// - If neither attribute is used, the original field name will be retained.
///
/// For structs with unnamed fields, the value representation will be [`Value::List`], with all
/// fields inserted into a list.
/// Unit structs will be represented as [`Value::Nothing`] since they contain no data.
///
/// For enums, the resulting value representation depends on the variant name:
/// - If `#[nu_value(rename = "...")]` is applied to a specific variant, that name is used.
/// - If `#[nu_value(rename_all = "...")]` is applied to the enum, variant names will be
///   case-converted accordingly.
/// - If neither attribute is used, variant names will default to snake_case.
///
/// Only enums with no fields may derive this trait.
/// The resulting value will be the name of the variant as a [`Value::String`].
///
/// All case options from [`heck`] are supported, as well as the values allowed by
/// [`#[serde(rename_all)]`](https://serde.rs/container-attrs.html#rename_all).
///
/// # Enum Example
/// ```
/// # use nu_protocol::{IntoValue, Value, Span, record};
/// #
/// # let span = Span::unknown();
/// #
/// #[derive(IntoValue)]
/// #[nu_value(rename_all = "COBOL-CASE")]
/// enum Bird {
///     MountainEagle,
///     ForestOwl,
///     #[nu_value(rename = "RIVER-QUACK")]
///     RiverDuck,
/// }
///
/// assert_eq!(
///     Bird::ForestOwl.into_value(span),
///     Value::string("FOREST-OWL", span)
/// );
///
/// assert_eq!(
///     Bird::RiverDuck.into_value(span),
///     Value::string("RIVER-QUACK", span)
/// );
/// ```
///
/// # Struct Example
/// ```
/// # use nu_protocol::{IntoValue, Value, Span, record};
/// #
/// # let span = Span::unknown();
/// #
/// #[derive(IntoValue)]
/// #[nu_value(rename_all = "kebab-case")]
/// struct Person {
///     first_name: String,
///     last_name: String,
///     #[nu_value(rename = "age")]
///     age_years: u32,
/// }
///
/// assert_eq!(
///     Person {
///         first_name: "John".into(),
///         last_name: "Doe".into(),
///         age_years: 42,
///     }.into_value(span),
///     Value::record(record! {
///         "first-name" => Value::string("John", span),
///         "last-name" => Value::string("Doe", span),
///         "age" => Value::int(42, span),
///     }, span)
/// );
/// ```
pub trait IntoValue: Sized {
    /// Converts the given value to a [`Value`].
    fn into_value(self, span: Span) -> Value;
}

// Primitive Types

impl<T, const N: usize> IntoValue for [T; N]
where
    T: IntoValue,
{
    fn into_value(self, span: Span) -> Value {
        Vec::from(self).into_value(span)
    }
}

macro_rules! primitive_into_value {
    ($type:ty, $method:ident) => {
        primitive_into_value!($type => $type, $method);
    };

    ($type:ty => $as_type:ty, $method:ident) => {
        impl IntoValue for $type {
            fn into_value(self, span: Span) -> Value {
                Value::$method(<$as_type>::from(self), span)
            }
        }
    };
}

primitive_into_value!(bool, bool);
primitive_into_value!(char, string);
primitive_into_value!(f32 => f64, float);
primitive_into_value!(f64, float);
primitive_into_value!(i8 => i64, int);
primitive_into_value!(i16 => i64, int);
primitive_into_value!(i32 => i64, int);
primitive_into_value!(i64, int);
primitive_into_value!(u8 => i64, int);
primitive_into_value!(u16 => i64, int);
primitive_into_value!(u32 => i64, int);
// u64 and usize may be truncated as Value only supports i64.

impl IntoValue for isize {
    fn into_value(self, span: Span) -> Value {
        Value::int(self as i64, span)
    }
}

impl IntoValue for () {
    fn into_value(self, span: Span) -> Value {
        Value::nothing(span)
    }
}

macro_rules! tuple_into_value {
    ($($t:ident:$n:tt),+) => {
        impl<$($t),+> IntoValue for ($($t,)+) where $($t: IntoValue,)+ {
            fn into_value(self, span: Span) -> Value {
                let vals = vec![$(self.$n.into_value(span)),+];
                Value::list(vals, span)
            }
        }
    }
}

// Tuples in std are implemented for up to 12 elements, so we do it here too.
tuple_into_value!(T0:0);
tuple_into_value!(T0:0, T1:1);
tuple_into_value!(T0:0, T1:1, T2:2);
tuple_into_value!(T0:0, T1:1, T2:2, T3:3);
tuple_into_value!(T0:0, T1:1, T2:2, T3:3, T4:4);
tuple_into_value!(T0:0, T1:1, T2:2, T3:3, T4:4, T5:5);
tuple_into_value!(T0:0, T1:1, T2:2, T3:3, T4:4, T5:5, T6:6);
tuple_into_value!(T0:0, T1:1, T2:2, T3:3, T4:4, T5:5, T6:6, T7:7);
tuple_into_value!(T0:0, T1:1, T2:2, T3:3, T4:4, T5:5, T6:6, T7:7, T8:8);
tuple_into_value!(T0:0, T1:1, T2:2, T3:3, T4:4, T5:5, T6:6, T7:7, T8:8, T9:9);
tuple_into_value!(T0:0, T1:1, T2:2, T3:3, T4:4, T5:5, T6:6, T7:7, T8:8, T9:9, T10:10);
tuple_into_value!(T0:0, T1:1, T2:2, T3:3, T4:4, T5:5, T6:6, T7:7, T8:8, T9:9, T10:10, T11:11);

// Other std Types

impl IntoValue for String {
    fn into_value(self, span: Span) -> Value {
        Value::string(self, span)
    }
}

impl IntoValue for &str {
    fn into_value(self, span: Span) -> Value {
        Value::string(self, span)
    }
}

impl<T> IntoValue for Vec<T>
where
    T: IntoValue,
{
    fn into_value(self, span: Span) -> Value {
        Value::list(self.into_iter().map(|v| v.into_value(span)).collect(), span)
    }
}

impl<T> IntoValue for Option<T>
where
    T: IntoValue,
{
    fn into_value(self, span: Span) -> Value {
        match self {
            Some(v) => v.into_value(span),
            None => Value::nothing(span),
        }
    }
}

/// This blanket implementation permits the use of [`Cow<'_, B>`] ([`Cow<'_, str>`] etc) based on
/// the [IntoValue] implementation of `B`'s owned form ([str] => [String]).
///
/// It's meant to make using the [IntoValue] derive macro on types that contain [Cow] fields
/// possible.
impl<B> IntoValue for Cow<'_, B>
where
    B: ?Sized + ToOwned,
    B::Owned: IntoValue,
{
    fn into_value(self, span: Span) -> Value {
        <B::Owned as IntoValue>::into_value(self.into_owned(), span)
    }
}

impl<K, V> IntoValue for HashMap<K, V>
where
    K: Borrow<str> + Into<String>,
    V: IntoValue,
{
    fn into_value(self, span: Span) -> Value {
        // The `Borrow<str>` constraint is to ensure uniqueness, as implementations of `Borrow`
        // must uphold by certain properties (e.g., `(x == y) == (x.borrow() == y.borrow())`.
        //
        // The `Into<String>` constraint is necessary for us to convert the key into a `String`.
        // Most types that implement `Borrow<str>` also implement `Into<String>`.
        // Implementations of `Into` must also be lossless and value-preserving conversions.
        // So, when combined with the `Borrow` constraint, this means that the converted
        // `String` keys should be unique.
        self.into_iter()
            .map(|(k, v)| (k.into(), v.into_value(span)))
            .collect::<Record>()
            .into_value(span)
    }
}

impl IntoValue for std::time::Duration {
    fn into_value(self, span: Span) -> Value {
        let val: u128 = self.as_nanos();
        debug_assert!(val <= i64::MAX as u128, "duration value too large");
        // Capping is the best effort here.
        let val: i64 = val.try_into().unwrap_or(i64::MAX);
        Value::duration(val, span)
    }
}

// Nu Types

impl IntoValue for Range {
    fn into_value(self, span: Span) -> Value {
        Value::range(self, span)
    }
}

impl IntoValue for Record {
    fn into_value(self, span: Span) -> Value {
        Value::record(self, span)
    }
}

impl IntoValue for Closure {
    fn into_value(self, span: Span) -> Value {
        Value::closure(self, span)
    }
}

impl IntoValue for ShellError {
    fn into_value(self, span: Span) -> Value {
        Value::error(self, span)
    }
}

impl IntoValue for CellPath {
    fn into_value(self, span: Span) -> Value {
        Value::cell_path(self, span)
    }
}

impl IntoValue for Value {
    fn into_value(self, span: Span) -> Value {
        self.with_span(span)
    }
}

// Foreign Types

impl IntoValue for DateTime<FixedOffset> {
    fn into_value(self, span: Span) -> Value {
        Value::date(self, span)
    }
}

impl IntoValue for bytes::Bytes {
    fn into_value(self, span: Span) -> Value {
        Value::binary(self.to_vec(), span)
    }
}

// TODO: use this type for all the `into_value` methods that types implement but return a Result
/// A trait for trying to convert a value into a `Value`.
///
/// Types like streams may fail while collecting the `Value`,
/// for these types it is useful to implement a fallible variant.
///
/// This conversion is fallible, for infallible conversions use [`IntoValue`].
/// All types that implement `IntoValue` will automatically implement this trait.
pub trait TryIntoValue: Sized {
    // TODO: instead of ShellError, maybe we could have a IntoValueError that implements Into<ShellError>
    /// Tries to convert the given value into a `Value`.
    fn try_into_value(self, span: Span) -> Result<Value, ShellError>;
}

impl<T> TryIntoValue for T
where
    T: IntoValue,
{
    fn try_into_value(self, span: Span) -> Result<Value, ShellError> {
        Ok(self.into_value(span))
    }
}
