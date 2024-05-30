use crate::{record, FromValue, IntoValue, Record, Span, Value};
use std::collections::HashMap;

// make nu_protocol available in this namespace, consumers of this crate will
// have this without such an export
// the derive macro fully qualifies paths to "nu_protocol"
use crate as nu_protocol;

macro_rules! make_type {
    (
        $(#[$meta:meta])*
        struct $name:ident {
            $($field:ident : $t:ty = $val:expr),* $(,)?
        }
    ) => {
        $(#[$meta])*
        struct $name {
            $($field: $t,)*
        }

        impl $name {
            fn make() -> Self {
                Self {
                    $($field: $val,)*
                }
            }
        }
    };
}

make_type! {
    #[derive(IntoValue, FromValue, Debug, PartialEq)]
    struct Primitives {
        p_array: [u16; 4] = [12, 34, 56, 78],
        p_bool: bool = true,
        p_char: char = 'A',
        p_f32: f32 = 123.456,
        p_f64: f64 = 789.1011,
        p_i8: i8 = -12,
        p_i16: i16 = -1234,
        p_i32: i32 = -123456,
        p_i64: i64 = -1234567890,
        p_isize: isize = 1024,
        p_str: String = "Hello, world!".to_string(),
        p_u16: u16 = 65535,
        p_u32: u32 = 4294967295,
        p_u64: u64 = 8446744073709551615,
        p_usize: usize = 4096,
        p_unit: () = (),
        p_tuple: (u32, bool) = (123456789, false),
    }
}

fn assert_record_field(value: &mut Record, key: &str, expected: Value) {
    let field = value
        .remove(key)
        .expect(&format!("expected record to have {key:?}"));
    assert_eq!(field, expected);
}

#[test]
fn primitives_into_value() {
    let primitives = Primitives::make();
    let mut record = primitives.into_value_unknown().into_record().unwrap();
    assert_record_field(
        &mut record,
        "p_array",
        Value::test_list(vec![
            Value::test_int(12),
            Value::test_int(34),
            Value::test_int(56),
            Value::test_int(78),
        ]),
    );
    assert_record_field(&mut record, "p_bool", Value::test_bool(true));
    assert_record_field(&mut record, "p_char", Value::test_string("A"));
    assert_record_field(&mut record, "p_f64", Value::test_float(789.1011));
    assert_record_field(&mut record, "p_i8", Value::test_int(-12));
    assert_record_field(&mut record, "p_i16", Value::test_int(-1234));
    assert_record_field(&mut record, "p_i32", Value::test_int(-123456));
    assert_record_field(&mut record, "p_i64", Value::test_int(-1234567890));
    assert_record_field(&mut record, "p_isize", Value::test_int(1024));
    assert_record_field(&mut record, "p_str", Value::test_string("Hello, world!"));
    assert_record_field(&mut record, "p_u16", Value::test_int(65535));
    assert_record_field(&mut record, "p_u32", Value::test_int(4294967295));
    assert_record_field(&mut record, "p_u64", Value::test_int(8446744073709551615));
    assert_record_field(&mut record, "p_usize", Value::test_int(4096));
    assert_record_field(&mut record, "p_unit", Value::test_nothing());
    assert_record_field(
        &mut record,
        "p_tuple",
        Value::test_list(vec![Value::test_int(123456789), Value::test_bool(false)]),
    );

    // Handle f32 separately to cast the value back down to f32 for comparison.
    let key = "p_f32";
    let p_f32 = record
        .remove(key)
        .expect("expected record to have {key:?}")
        .as_float()
        .expect("{key:?} was not a float");
    assert_eq!(p_f32 as f32, 123.456);

    assert!(record.is_empty());
}

#[test]
fn primitives_from_value() {
    let value = Value::test_record(record! {
        "p_array" => Value::test_list(vec![
            Value::test_int(12),
            Value::test_int(34),
            Value::test_int(56),
            Value::test_int(78),
        ]),
        "p_bool" => Value::test_bool(true),
        "p_char" => Value::test_string('A'),
        "p_f32" => Value::test_float(123.456),
        "p_f64" => Value::test_float(789.1011),
        "p_i8" => Value::test_int(-12),
        "p_i16" => Value::test_int(-1234),
        "p_i32" => Value::test_int(-123456),
        "p_i64" => Value::test_int(-1234567890),
        "p_isize" => Value::test_int(1024),
        "p_str" => Value::test_string("Hello, world!"),
        "p_u16" => Value::test_int(65535),
        "p_u32" => Value::test_int(4294967295),
        "p_u64" => Value::test_int(8446744073709551615),
        "p_usize" => Value::test_int(4096),
        "p_unit" => Value::test_nothing(),
        "p_tuple" => Value::test_list(vec![
            Value::test_int(123456789),
            Value::test_bool(false)
        ]),
    });
    let expected = Primitives::make();
    let actual = Primitives::from_value(value, Span::unknown()).unwrap();
    assert_eq!(expected, actual);
}

make_type! {
    #[derive(IntoValue, FromValue, Debug, PartialEq)]
    struct StdValues {
        some: Option<usize> = Some(123),
        none: Option<usize> = None,
        vec: Vec<usize> = vec![1, 2],
        string: String = "Hello std!".to_string(),
        hashmap: HashMap<String, Value> = HashMap::from([
            ("int".to_string(), Value::test_int(123)),
            ("str".to_string(), Value::test_string("some value")),
            ("bool".to_string(), Value::test_bool(true))
        ]),
    }
}

#[test]
fn std_values_into_value() {
    let actual = StdValues::make().into_value_unknown();
    let expected = Value::test_record(record! {
        "some" => Value::test_int(123),
        "none" => Value::test_nothing(),
        "vec" => Value::test_list(vec![Value::test_int(1), Value::test_int(2)]),
        "string" => Value::test_string("Hello std!"),
        "hashmap" => Value::test_record(record! {
            "int" => Value::test_int(123),
            "str" => Value::test_string("some value"),
            "bool" => Value::test_bool(true)
        }),
    });
    assert_eq!(actual, expected);
}

#[test]
fn std_values_from_value() {
    let value = Value::test_record(record! {
        "some" => Value::test_int(123),
        "none" => Value::test_nothing(),
        "vec" => Value::test_list(vec![Value::test_int(1), Value::test_int(2)]),
        "string" => Value::test_string("Hello std!"),
        "hashmap" => Value::test_record(record! {
            "int" => Value::test_int(123),
            "str" => Value::test_string("some value"),
            "bool" => Value::test_bool(true)
        })
    });
    let actual = StdValues::from_value(value, Span::unknown()).unwrap();
    let expected = StdValues::make();
    assert_eq!(actual, expected);
}

make_type! {
    #[derive(IntoValue)]
    struct Outer {
        a: InnerA = InnerA { d: true },
        b: InnerB = InnerB { e: 123.456, f: () },
        c: u8 = 69,
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
    let nested = Outer::make().into_value_unknown();
    let expected = Value::test_record(record! {
        "a" => Value::test_record(record! {
            "d" => Value::test_bool(true),
        }),
        "b" => Value::test_record(record! {
            "e" => Value::test_float(123.456),
            "f" => Value::test_nothing(),
        }),
        "c" => Value::test_int(69),
    });
    assert_eq!(nested, expected);
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
