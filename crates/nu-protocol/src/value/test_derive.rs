use crate::{record, FromValue, IntoValue, Record, Value};
use std::collections::HashMap;

// Make nu_protocol available in this namespace, consumers of this crate will
// have this without such an export.
// The derive macro fully qualifies paths to "nu_protocol".
use crate as nu_protocol;

#[derive(IntoValue, FromValue, Debug, PartialEq)]
struct NamedFieldsStruct<T>
where
    T: IntoValue + FromValue,
{
    array: [u16; 4],
    bool: bool,
    char: char,
    f32: f32,
    f64: f64,
    i8: i8,
    i16: i16,
    i32: i32,
    i64: i64,
    isize: isize,
    u16: u16,
    u32: u32,
    unit: (),
    tuple: (u32, bool),
    some: Option<u32>,
    none: Option<u32>,
    vec: Vec<T>,
    string: String,
    hashmap: HashMap<String, u32>,
    nested: Nestee,
}

#[derive(IntoValue, FromValue, Debug, PartialEq)]
struct Nestee {
    u32: u32,
    some: Option<u32>,
    none: Option<u32>,
}

impl NamedFieldsStruct<u32> {
    fn make() -> Self {
        Self {
            array: [1, 2, 3, 4],
            bool: true,
            char: 'a',
            f32: std::f32::consts::PI,
            f64: std::f64::consts::E,
            i8: 127,
            i16: -32768,
            i32: 2147483647,
            i64: -9223372036854775808,
            isize: 2,
            u16: 65535,
            u32: 4294967295,
            unit: (),
            tuple: (1, true),
            some: Some(123),
            none: None,
            vec: vec![10, 20, 30],
            string: "string".to_string(),
            hashmap: HashMap::from_iter([("a".to_string(), 10), ("b".to_string(), 20)]),
            nested: Nestee {
                u32: 3,
                some: Some(42),
                none: None,
            },
        }
    }

    fn value() -> Value {
        Value::test_record(record! {
            "array" => Value::test_list(vec![
                Value::test_int(1),
                Value::test_int(2),
                Value::test_int(3),
                Value::test_int(4)
            ]),
            "bool" => Value::test_bool(true),
            "char" => Value::test_string('a'),
            "f32" => Value::test_float(std::f32::consts::PI as f64),
            "f64" => Value::test_float(std::f64::consts::E),
            "i8" => Value::test_int(127),
            "i16" => Value::test_int(-32768),
            "i32" => Value::test_int(2147483647),
            "i64" => Value::test_int(-9223372036854775808),
            "isize" => Value::test_int(2),
            "u16" => Value::test_int(65535),
            "u32" => Value::test_int(4294967295),
            "unit" => Value::test_nothing(),
            "tuple" => Value::test_list(vec![
                Value::test_int(1),
                Value::test_bool(true)
            ]),
            "some" => Value::test_int(123),
            "none" => Value::test_nothing(),
            "vec" => Value::test_list(vec![
                Value::test_int(10),
                Value::test_int(20),
                Value::test_int(30)
            ]),
            "string" => Value::test_string("string"),
            "hashmap" => Value::test_record(record! {
                "a" => Value::test_int(10),
                "b" => Value::test_int(20)
            }),
            "nested" => Value::test_record(record! {
                "u32" => Value::test_int(3),
                "some" => Value::test_int(42),
                "none" => Value::test_nothing(),
            })
        })
    }
}

#[test]
fn named_fields_struct_into_value() {
    let expected = NamedFieldsStruct::value();
    let actual = NamedFieldsStruct::make().into_value_unknown();
    assert_eq!(expected, actual);
}

#[test]
fn named_fields_struct_from_value() {
    let expected = NamedFieldsStruct::make();
    let actual = NamedFieldsStruct::from_value(NamedFieldsStruct::value()).unwrap();
    assert_eq!(expected, actual);
}

#[test]
fn named_fields_struct_roundtrip() {
    let expected = NamedFieldsStruct::make();
    let actual =
        NamedFieldsStruct::from_value(NamedFieldsStruct::make().into_value_unknown()).unwrap();
    assert_eq!(expected, actual);

    let expected = NamedFieldsStruct::value();
    let actual = NamedFieldsStruct::<u32>::from_value(NamedFieldsStruct::value())
        .unwrap()
        .into_value_unknown();
    assert_eq!(expected, actual);
}

#[test]
fn named_fields_struct_missing_value() {
    let value = Value::test_record(Record::new());
    let res: Result<NamedFieldsStruct<u32>, _> = NamedFieldsStruct::from_value(value);
    assert!(res.is_err());
}

#[test]
fn named_fields_struct_incorrect_type() {
    // Should work for every type that is not a record.
    let value = Value::test_nothing();
    let res: Result<NamedFieldsStruct<u32>, _> = NamedFieldsStruct::from_value(value);
    assert!(res.is_err());
}

#[derive(IntoValue, FromValue, Debug, PartialEq)]
struct UnnamedFieldsStruct<T>(u32, String, T)
where
    T: IntoValue + FromValue;

impl UnnamedFieldsStruct<f64> {
    fn make() -> Self {
        UnnamedFieldsStruct(420, "Hello, tuple!".to_string(), 33.33)
    }

    fn value() -> Value {
        Value::test_list(vec![
            Value::test_int(420),
            Value::test_string("Hello, tuple!"),
            Value::test_float(33.33),
        ])
    }
}

#[test]
fn unnamed_fields_struct_into_value() {
    let expected = UnnamedFieldsStruct::value();
    let actual = UnnamedFieldsStruct::make().into_value_unknown();
    assert_eq!(expected, actual);
}

#[test]
fn unnamed_fields_struct_from_value() {
    let expected = UnnamedFieldsStruct::make();
    let value = UnnamedFieldsStruct::value();
    let actual = UnnamedFieldsStruct::from_value(value).unwrap();
    assert_eq!(expected, actual);
}

#[test]
fn unnamed_fields_struct_roundtrip() {
    let expected = UnnamedFieldsStruct::make();
    let actual =
        UnnamedFieldsStruct::from_value(UnnamedFieldsStruct::make().into_value_unknown()).unwrap();
    assert_eq!(expected, actual);

    let expected = UnnamedFieldsStruct::value();
    let actual = UnnamedFieldsStruct::<f64>::from_value(UnnamedFieldsStruct::value())
        .unwrap()
        .into_value_unknown();
    assert_eq!(expected, actual);
}

#[test]
fn unnamed_fields_struct_missing_value() {
    let value = Value::test_list(vec![]);
    let res: Result<UnnamedFieldsStruct<f64>, _> = UnnamedFieldsStruct::from_value(value);
    assert!(res.is_err());
}

#[test]
fn unnamed_fields_struct_incorrect_type() {
    // Should work for every type that is not a record.
    let value = Value::test_nothing();
    let res: Result<UnnamedFieldsStruct<f64>, _> = UnnamedFieldsStruct::from_value(value);
    assert!(res.is_err());
}

#[derive(IntoValue, FromValue, Debug, PartialEq)]
struct UnitStruct;

#[test]
fn unit_struct_into_value() {
    let expected = Value::test_nothing();
    let actual = UnitStruct.into_value_unknown();
    assert_eq!(expected, actual);
}

#[test]
fn unit_struct_from_value() {
    let expected = UnitStruct;
    let actual = UnitStruct::from_value(Value::test_nothing()).unwrap();
    assert_eq!(expected, actual);
}

#[test]
fn unit_struct_roundtrip() {
    let expected = UnitStruct;
    let actual = UnitStruct::from_value(UnitStruct.into_value_unknown()).unwrap();
    assert_eq!(expected, actual);

    let expected = Value::test_nothing();
    let actual = UnitStruct::from_value(Value::test_nothing())
        .unwrap()
        .into_value_unknown();
    assert_eq!(expected, actual);
}

#[derive(IntoValue, FromValue, Debug, PartialEq)]
enum Enum {
    AlphaOne,
    BetaTwo,
    CharlieThree,
}

impl Enum {
    fn make() -> [Self; 3] {
        [Enum::AlphaOne, Enum::BetaTwo, Enum::CharlieThree]
    }

    fn value() -> Value {
        Value::test_list(vec![
            Value::test_string("alpha_one"),
            Value::test_string("beta_two"),
            Value::test_string("charlie_three"),
        ])
    }
}

#[test]
fn enum_into_value() {
    let expected = Enum::value();
    let actual = Enum::make().into_value_unknown();
    assert_eq!(expected, actual);
}

#[test]
fn enum_from_value() {
    let expected = Enum::make();
    let actual = <[Enum; 3]>::from_value(Enum::value()).unwrap();
    assert_eq!(expected, actual);
}

#[test]
fn enum_roundtrip() {
    let expected = Enum::make();
    let actual = <[Enum; 3]>::from_value(Enum::make().into_value_unknown()).unwrap();
    assert_eq!(expected, actual);

    let expected = Enum::value();
    let actual = <[Enum; 3]>::from_value(Enum::value())
        .unwrap()
        .into_value_unknown();
    assert_eq!(expected, actual);
}

#[test]
fn enum_unknown_variant() {
    let value = Value::test_string("delta_four");
    let res = Enum::from_value(value);
    assert!(res.is_err());
}

#[test]
fn enum_incorrect_type() {
    // Should work for every type that is not a record.
    let value = Value::test_nothing();
    let res = Enum::from_value(value);
    assert!(res.is_err());
}
