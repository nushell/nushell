use std::collections::HashMap;

use crate::{Record, ShellError, Span, Value};

/// A trait for converting a value into a [`Value`].
///
/// This conversion is infallible, for fallible conversions use [`TryIntoValue`].
///
/// # Derivable
/// This trait can be used with `#[derive]`.
/// When derived on structs with named fields, the resulting value representation will use
/// [`Value::Record`], where each field of the record corresponds to a field of the struct.
/// For structs with unnamed fields, the value representation will be [`Value::List`], with all
/// fields inserted into a list.
/// Unit structs will be represented as [`Value::Nothing`] since they contain no data.
///
/// Only enums with no fields may derive this trait.
/// The resulting value representation will be the name of the variant as a [`Value::String`].
/// By default, variant names will be converted to ["snake_case"](convert_case::Case::Snake).
/// You can customize the case conversion using `#[nu_value(rename_all = "kebab-case")]` on the enum.
/// All deterministic and useful case conversions provided by [`convert_case::Case`] are supported
/// by specifying the case name followed by "case".
/// Also all values for
/// [`#[serde(rename_all = "...")]`](https://serde.rs/container-attrs.html#rename_all) are valid
/// here.
///
/// ```
/// # use nu_protocol::{IntoValue, Value, Span};
/// #[derive(IntoValue)]
/// #[nu_value(rename_all = "COBOL-CASE")]
/// enum Bird {
///     MountainEagle,
///     ForestOwl,
///     RiverDuck,
/// }
///
/// assert_eq!(
///     Bird::RiverDuck.into_value(Span::unknown()),
///     Value::test_string("RIVER-DUCK")
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

impl<V> IntoValue for HashMap<String, V>
where
    V: IntoValue,
{
    fn into_value(self, span: Span) -> Value {
        let mut record = Record::new();
        for (k, v) in self.into_iter() {
            // Using `push` is fine as a hashmaps have unique keys.
            // To ensure this uniqueness, we only allow hashmaps with strings as
            // keys and not keys which implement `Into<String>` or `ToString`.
            record.push(k, v.into_value(span));
        }
        Value::record(record, span)
    }
}

// Nu Types

impl IntoValue for Value {
    fn into_value(self, span: Span) -> Value {
        self.with_span(span)
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
