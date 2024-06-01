use crate::{record, FromValue, IntoValue, Record, Span, Value};
use std::collections::HashMap;

// Make nu_protocol available in this namespace, consumers of this crate will
// have this without such an export.
// The derive macro fully qualifies paths to "nu_protocol".
use crate as nu_protocol;

macro_rules! make_struct {
    (
        $(#[$meta:meta])*
        struct $name:ident {
            $($field:ident : $t:ty = $raw:expr, $val:expr),* $(,)?
        }
    ) => {
        $(#[$meta])*
        struct $name {
            $($field: $t,)*
        }

        impl $name {
            fn make() -> Self {
                Self {
                    $($field: $raw,)*
                }
            }

            fn value() -> Value {
                Value::test_record(record! {
                    $(stringify!($field) => $val),*
                })
            }
        }
    };
}

make_struct! {
    #[derive(IntoValue, FromValue, Debug, PartialEq)]
    struct Primitives {
        p_array: [u16; 4] = [12, 34, 56, 78], Value::test_list(vec![
            Value::test_int(12), 
            Value::test_int(34), 
            Value::test_int(56), 
            Value::test_int(78),
        ]),
        p_bool: bool = true, Value::test_bool(true),
        p_char: char = 'A', Value::test_string('A'),
        p_f32: f32 = 123.456, Value::test_float(123.456f32 as f64),
        p_f64: f64 = 789.1011, Value::test_float(789.1011),
        p_i8: i8 = -12, Value::test_int(-12),
        p_i16: i16 = -1234, Value::test_int(-1234),
        p_i32: i32 = -123456, Value::test_int(-123456),
        p_i64: i64 = -1234567890, Value::test_int(-1234567890),
        p_isize: isize = 1024, Value::test_int(1024),
        p_str: String = "Hello, world!".to_string(), Value::test_string("Hello, world!"),
        p_u16: u16 = 65535, Value::test_int(65535),
        p_u32: u32 = 4294967295, Value::test_int(4294967295),
        p_u64: u64 = 8446744073709551615, Value::test_int(8446744073709551615),
        p_usize: usize = 4096, Value::test_int(4096),
        p_unit: () = (), Value::test_nothing(),
        p_tuple: (u32, bool) = (123456789, false), Value::test_list(vec![
            Value::test_int(123456789), 
            Value::test_bool(false),
        ])
    }
}

#[test]
fn primitives_into_value() {
    let expected = Primitives::value();
    let actual = Primitives::make().into_value_unknown();
    assert_eq!(expected, actual);
}

#[test]
fn primitives_from_value() {
    let expected = Primitives::make();
    let actual = Primitives::from_value(Primitives::value(), Span::test_data()).unwrap();
    assert_eq!(expected, actual);
}

make_struct! {
    #[derive(IntoValue, FromValue, Debug, PartialEq)]
    struct StdValues {
        some: Option<usize> = Some(123), Value::test_int(123),
        none: Option<usize> = None, Value::test_nothing(),
        vec: Vec<usize> = vec![1, 2], Value::test_list(vec![
            Value::test_int(1), 
            Value::test_int(2),
        ]),
        string: String = "Hello std!".to_string(), Value::test_string("Hello std!"),
        hashmap: HashMap<String, Value> = HashMap::from([
            ("int".to_string(), Value::test_int(123)),
            ("str".to_string(), Value::test_string("some value")),
            ("bool".to_string(), Value::test_bool(true))
        ]), Value::test_record(record! {
            "int" => Value::test_int(123),
            "str" => Value::test_string("some value"),
            "bool" => Value::test_bool(true),
        })
    }
}

#[test]
fn std_values_into_value() {
    let expected = StdValues::value();
    let actual = StdValues::make().into_value_unknown();
    assert_eq!(expected, actual);
}

#[test]
fn std_values_from_value() {
    let expected = StdValues::make();
    let actual = StdValues::from_value(StdValues::value(), Span::test_data()).unwrap();
    assert_eq!(expected, actual);
}

make_struct! {
    #[derive(IntoValue)]
    struct Outer {
        a: InnerA = InnerA { d: true }, Value::test_record(record! {
            "d" => Value::test_bool(true),
        }),
        b: InnerB = InnerB { e: 123.456, f: () }, Value::test_record(record! {
            "e" => Value::test_float(123.456),
            "f" => Value::test_nothing(),
        }),
        c: u8 = 69, Value::test_int(69),
    }
}

#[derive(IntoValue)]
struct InnerA {
    d: bool,
}

#[derive(IntoValue)]
struct InnerB {
    e: f64,
    f: (),
}

#[test]
fn nested_into_value() {
    let expected = Outer::value();
    let actual = Outer::make().into_value_unknown();
    assert_eq!(expected, actual);
}

#[derive(IntoValue)]
struct TupleStruct(usize, String, f64);

impl TupleStruct {
    fn make() -> Self {
        TupleStruct(420, "Hello, tuple!".to_string(), 33.33)
    }
}

#[test]
fn tuple_struct_into_value() {
    let tuple = TupleStruct::make().into_value_unknown();
    let expected = Value::test_list(vec![
        Value::test_int(420),
        Value::test_string("Hello, tuple!"),
        Value::test_float(33.33),
    ]);
    assert_eq!(tuple, expected);
}

#[derive(IntoValue)]
struct Unit;

#[test]
fn unit_into_value() {
    let unit = Unit.into_value_unknown();
    let expected = Value::test_nothing();
    assert_eq!(unit, expected);
}

#[derive(IntoValue)]
enum Enum {
    Unit,
    Tuple(u32, String),
    Struct { a: u32, b: String },
}

impl Enum {
    fn make() -> [Self; 3] {
        [
            Enum::Unit,
            Enum::Tuple(12, "Tuple variant".to_string()),
            Enum::Struct {
                a: 34,
                b: "Struct variant".to_string(),
            },
        ]
    }
}

#[test]
fn enum_into_value() {
    let enums = Enum::make().into_value_unknown();
    let expected = Value::test_list(vec![
        Value::test_record(record! {
            "type" => Value::test_string("unit")
        }),
        Value::test_record(record! {
            "type" => Value::test_string("tuple"),
            "content" => Value::test_list(vec![
                Value::test_int(12),
                Value::test_string("Tuple variant")
            ])
        }),
        Value::test_record(record! {
            "type" => Value::test_string("struct"),
            "content" => Value::test_record(record! {
                "a" => Value::test_int(34),
                "b" => Value::test_string("Struct variant")
            })
        }),
    ]);
    assert_eq!(enums, expected);
}
