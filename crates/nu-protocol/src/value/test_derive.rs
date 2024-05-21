use crate::{record, IntoValue, Record, Value};

// make nu_protocol available in this namespace, consumers of this crate will
// have this without such an export
// the derive macro fully qualifies paths to "nu_protocol"
use crate as nu_protocol;

#[derive(IntoValue)]
struct Primitives {
    p_array: [u8; 4],
    p_bool: bool,
    p_char: char,
    p_f32: f32,
    p_f64: f64,
    p_i8: i8,
    p_i16: i16,
    p_i32: i32,
    p_i64: i64,
    p_isize: isize,
    p_str: &'static str,
    p_u8: u8,
    p_u16: u16,
    p_u32: u32,
    p_u64: u64,
    p_usize: usize,
    p_unit: (),
    p_tuple: (u32, bool),
}

impl Primitives {
    fn make() -> Primitives {
        Primitives {
            p_array: [12, 34, 56, 78],
            p_bool: true,
            p_char: 'A',
            p_f32: 123.456,
            p_f64: 789.1011,
            p_i8: -12,
            p_i16: -1234,
            p_i32: -123456,
            p_i64: -1234567890,
            p_isize: 1024,
            p_str: "Hello, world!",
            p_u8: 255,
            p_u16: 65535,
            p_u32: 4294967295,
            p_u64: 8446744073709551615,
            p_usize: 4096,
            p_unit: (),
            p_tuple: (123456789, false),
        }
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
    assert_record_field(&mut record, "p_u8", Value::test_int(255));
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

#[derive(IntoValue)]
struct StdValues {
    some: Option<usize>,
    none: Option<usize>,
    vec: Vec<usize>,
    string: String,
}

impl StdValues {
    fn make() -> Self {
        StdValues {
            some: Some(123),
            none: None,
            vec: vec![1, 2],
            string: "Hello std!".to_string(),
        }
    }
}

#[test]
fn std_values_into_value() {
    let actual = StdValues::make().into_value_unknown();
    let expected = Value::test_record(record! {
        "some" => Value::test_int(123),
        "none" => Value::test_nothing(),
        "vec" => Value::test_list(vec![Value::test_int(1), Value::test_int(2)]),
        "string" => Value::test_string("Hello std!")
    });
    assert_eq!(actual, expected);
}

#[derive(IntoValue)]
struct Outer {
    a: InnerA,
    b: InnerB,
    c: u8,
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

impl Outer {
    fn make() -> Self {
        Outer {
            a: InnerA { d: true },
            b: InnerB { e: 123.456, f: () },
            c: 69,
        }
    }
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
