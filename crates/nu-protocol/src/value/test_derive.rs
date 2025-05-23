use crate::{FromValue, IntoValue, Record, Span, Value, record};
use bytes::Bytes;
use std::collections::HashMap;

// Make nu_protocol available in this namespace, consumers of this crate will
// have this without such an export.
// The derive macro fully qualifies paths to "nu_protocol".
use crate as nu_protocol;

trait IntoTestValue {
    fn into_test_value(self) -> Value;
}

impl<T> IntoTestValue for T
where
    T: IntoValue,
{
    fn into_test_value(self) -> Value {
        self.into_value(Span::test_data())
    }
}

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
            "f32" => Value::test_float(std::f32::consts::PI.into()),
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
    let actual = NamedFieldsStruct::make().into_test_value();
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
        NamedFieldsStruct::from_value(NamedFieldsStruct::make().into_test_value()).unwrap();
    assert_eq!(expected, actual);

    let expected = NamedFieldsStruct::value();
    let actual = NamedFieldsStruct::<u32>::from_value(NamedFieldsStruct::value())
        .unwrap()
        .into_test_value();
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

#[derive(IntoValue, FromValue, Debug, PartialEq, Default)]
struct ALotOfOptions {
    required: bool,
    float: Option<f64>,
    int: Option<i64>,
    value: Option<Value>,
    nested: Option<Nestee>,
}

#[test]
fn missing_options() {
    let value = Value::test_record(Record::new());
    let res: Result<ALotOfOptions, _> = ALotOfOptions::from_value(value);
    assert!(res.is_err());

    let value = Value::test_record(record! {"required" => Value::test_bool(true)});
    let expected = ALotOfOptions {
        required: true,
        ..Default::default()
    };
    let actual = ALotOfOptions::from_value(value).unwrap();
    assert_eq!(expected, actual);

    let value = Value::test_record(record! {
        "required" => Value::test_bool(true),
        "float" => Value::test_float(std::f64::consts::PI),
    });
    let expected = ALotOfOptions {
        required: true,
        float: Some(std::f64::consts::PI),
        ..Default::default()
    };
    let actual = ALotOfOptions::from_value(value).unwrap();
    assert_eq!(expected, actual);

    let value = Value::test_record(record! {
        "required" => Value::test_bool(true),
        "int" => Value::test_int(12),
        "nested" => Value::test_record(record! {
            "u32" => Value::test_int(34),
        }),
    });
    let expected = ALotOfOptions {
        required: true,
        int: Some(12),
        nested: Some(Nestee {
            u32: 34,
            some: None,
            none: None,
        }),
        ..Default::default()
    };
    let actual = ALotOfOptions::from_value(value).unwrap();
    assert_eq!(expected, actual);
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
    let actual = UnnamedFieldsStruct::make().into_test_value();
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
        UnnamedFieldsStruct::from_value(UnnamedFieldsStruct::make().into_test_value()).unwrap();
    assert_eq!(expected, actual);

    let expected = UnnamedFieldsStruct::value();
    let actual = UnnamedFieldsStruct::<f64>::from_value(UnnamedFieldsStruct::value())
        .unwrap()
        .into_test_value();
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
    let actual = UnitStruct.into_test_value();
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
    let actual = UnitStruct::from_value(UnitStruct.into_test_value()).unwrap();
    assert_eq!(expected, actual);

    let expected = Value::test_nothing();
    let actual = UnitStruct::from_value(Value::test_nothing())
        .unwrap()
        .into_test_value();
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
    let actual = Enum::make().into_test_value();
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
    let actual = <[Enum; 3]>::from_value(Enum::make().into_test_value()).unwrap();
    assert_eq!(expected, actual);

    let expected = Enum::value();
    let actual = <[Enum; 3]>::from_value(Enum::value())
        .unwrap()
        .into_test_value();
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

mod enum_rename_all {
    use super::*;
    use crate as nu_protocol;

    // Generate the `Enum` from before but with all possible `rename_all` variants.
    macro_rules! enum_rename_all {
        ($($ident:ident: $case:literal => [$a1:literal, $b2:literal, $c3:literal]),*) => {
            $(
                #[derive(Debug, PartialEq, IntoValue, FromValue)]
                #[nu_value(rename_all = $case)]
                enum $ident {
                    AlphaOne,
                    BetaTwo,
                    CharlieThree
                }

                impl $ident {
                    fn make() -> [Self; 3] {
                        [Self::AlphaOne, Self::BetaTwo, Self::CharlieThree]
                    }

                    fn value() -> Value {
                        Value::test_list(vec![
                            Value::test_string($a1),
                            Value::test_string($b2),
                            Value::test_string($c3),
                        ])
                    }
                }
            )*

            #[test]
            fn into_value() {$({
                let expected = $ident::value();
                let actual = $ident::make().into_test_value();
                assert_eq!(expected, actual);
            })*}

            #[test]
            fn from_value() {$({
                let expected = $ident::make();
                let actual = <[$ident; 3]>::from_value($ident::value()).unwrap();
                assert_eq!(expected, actual);
            })*}
        }
    }

    enum_rename_all! {
        Upper: "UPPER CASE" => ["ALPHA ONE", "BETA TWO", "CHARLIE THREE"],
        Lower: "lower case" => ["alpha one", "beta two", "charlie three"],
        Title: "Title Case" => ["Alpha One", "Beta Two", "Charlie Three"],
        Camel: "camelCase" => ["alphaOne", "betaTwo", "charlieThree"],
        Pascal: "PascalCase" => ["AlphaOne", "BetaTwo", "CharlieThree"],
        Snake: "snake_case" => ["alpha_one", "beta_two", "charlie_three"],
        UpperSnake: "UPPER_SNAKE_CASE" => ["ALPHA_ONE", "BETA_TWO", "CHARLIE_THREE"],
        Kebab: "kebab-case" => ["alpha-one", "beta-two", "charlie-three"],
        Cobol: "COBOL-CASE" => ["ALPHA-ONE", "BETA-TWO", "CHARLIE-THREE"],
        Train: "Train-Case" => ["Alpha-One", "Beta-Two", "Charlie-Three"],
        Flat: "flatcase" => ["alphaone", "betatwo", "charliethree"],
        UpperFlat: "UPPERFLATCASE" => ["ALPHAONE", "BETATWO", "CHARLIETHREE"]
    }
}

mod named_fields_struct_rename_all {
    use super::*;
    use crate as nu_protocol;

    macro_rules! named_fields_struct_rename_all {
        ($($ident:ident: $case:literal => [$a1:literal, $b2:literal, $c3:literal]),*) => {
            $(
                #[derive(Debug, PartialEq, IntoValue, FromValue)]
                #[nu_value(rename_all = $case)]
                struct $ident {
                    alpha_one: (),
                    beta_two: (),
                    charlie_three: (),
                }

                impl $ident {
                    fn make() -> Self {
                        Self {
                            alpha_one: (),
                            beta_two: (),
                            charlie_three: (),
                        }
                    }

                    fn value() -> Value {
                        Value::test_record(record! {
                            $a1 => Value::test_nothing(),
                            $b2 => Value::test_nothing(),
                            $c3 => Value::test_nothing(),
                        })
                    }
                }
            )*

            #[test]
            fn into_value() {$({
                let expected = $ident::value();
                let actual = $ident::make().into_test_value();
                assert_eq!(expected, actual);
            })*}

            #[test]
            fn from_value() {$({
                let expected = $ident::make();
                let actual = $ident::from_value($ident::value()).unwrap();
                assert_eq!(expected, actual);
            })*}
        }
    }

    named_fields_struct_rename_all! {
        Upper: "UPPER CASE" => ["ALPHA ONE", "BETA TWO", "CHARLIE THREE"],
        Lower: "lower case" => ["alpha one", "beta two", "charlie three"],
        Title: "Title Case" => ["Alpha One", "Beta Two", "Charlie Three"],
        Camel: "camelCase" => ["alphaOne", "betaTwo", "charlieThree"],
        Pascal: "PascalCase" => ["AlphaOne", "BetaTwo", "CharlieThree"],
        Snake: "snake_case" => ["alpha_one", "beta_two", "charlie_three"],
        UpperSnake: "UPPER_SNAKE_CASE" => ["ALPHA_ONE", "BETA_TWO", "CHARLIE_THREE"],
        Kebab: "kebab-case" => ["alpha-one", "beta-two", "charlie-three"],
        Cobol: "COBOL-CASE" => ["ALPHA-ONE", "BETA-TWO", "CHARLIE-THREE"],
        Train: "Train-Case" => ["Alpha-One", "Beta-Two", "Charlie-Three"],
        Flat: "flatcase" => ["alphaone", "betatwo", "charliethree"],
        UpperFlat: "UPPERFLATCASE" => ["ALPHAONE", "BETATWO", "CHARLIETHREE"]
    }
}

#[derive(IntoValue, FromValue, Debug, PartialEq)]
struct ByteContainer {
    vec: Vec<u8>,
    bytes: Bytes,
}

impl ByteContainer {
    fn make() -> Self {
        ByteContainer {
            vec: vec![1, 2, 3],
            bytes: Bytes::from_static(&[4, 5, 6]),
        }
    }

    fn value() -> Value {
        Value::test_record(record! {
            "vec" => Value::test_list(vec![
                Value::test_int(1),
                Value::test_int(2),
                Value::test_int(3),
            ]),
            "bytes" => Value::test_binary(vec![4, 5, 6]),
        })
    }
}

#[test]
fn bytes_into_value() {
    let expected = ByteContainer::value();
    let actual = ByteContainer::make().into_test_value();
    assert_eq!(expected, actual);
}

#[test]
fn bytes_from_value() {
    let expected = ByteContainer::make();
    let actual = ByteContainer::from_value(ByteContainer::value()).unwrap();
    assert_eq!(expected, actual);
}

#[test]
fn bytes_roundtrip() {
    let expected = ByteContainer::make();
    let actual = ByteContainer::from_value(ByteContainer::make().into_test_value()).unwrap();
    assert_eq!(expected, actual);

    let expected = ByteContainer::value();
    let actual = ByteContainer::from_value(ByteContainer::value())
        .unwrap()
        .into_test_value();
    assert_eq!(expected, actual);
}

#[test]
fn struct_type_name_attr() {
    #[derive(FromValue, Debug)]
    #[nu_value(type_name = "struct")]
    struct TypeNameStruct;

    assert_eq!(
        TypeNameStruct::expected_type().to_string().as_str(),
        "struct"
    );
}

#[test]
fn enum_type_name_attr() {
    #[derive(FromValue, Debug)]
    #[nu_value(type_name = "enum")]
    struct TypeNameEnum;

    assert_eq!(TypeNameEnum::expected_type().to_string().as_str(), "enum");
}

#[derive(IntoValue, FromValue, Default, Debug, PartialEq)]
struct RenamedFieldStruct {
    #[nu_value(rename = "renamed")]
    field: (),
}

impl RenamedFieldStruct {
    fn value() -> Value {
        Value::test_record(record! {
            "renamed" => Value::test_nothing(),
        })
    }
}

#[test]
fn renamed_field_struct_into_value() {
    let expected = RenamedFieldStruct::value();
    let actual = RenamedFieldStruct::default().into_test_value();
    assert_eq!(expected, actual);
}

#[test]
fn renamed_field_struct_from_value() {
    let expected = RenamedFieldStruct::default();
    let actual = RenamedFieldStruct::from_value(RenamedFieldStruct::value()).unwrap();
    assert_eq!(expected, actual);
}

#[test]
fn renamed_field_struct_roundtrip() {
    let expected = RenamedFieldStruct::default();
    let actual =
        RenamedFieldStruct::from_value(RenamedFieldStruct::default().into_test_value()).unwrap();
    assert_eq!(expected, actual);

    let expected = RenamedFieldStruct::value();
    let actual = RenamedFieldStruct::from_value(RenamedFieldStruct::value())
        .unwrap()
        .into_test_value();
    assert_eq!(expected, actual);
}

#[derive(IntoValue, FromValue, Default, Debug, PartialEq)]
enum RenamedVariantEnum {
    #[default]
    #[nu_value(rename = "renamed")]
    Variant,
}

impl RenamedVariantEnum {
    fn value() -> Value {
        Value::test_string("renamed")
    }
}

#[test]
fn renamed_variant_enum_into_value() {
    let expected = RenamedVariantEnum::value();
    let actual = RenamedVariantEnum::default().into_test_value();
    assert_eq!(expected, actual);
}

#[test]
fn renamed_variant_enum_from_value() {
    let expected = RenamedVariantEnum::default();
    let actual = RenamedVariantEnum::from_value(RenamedVariantEnum::value()).unwrap();
    assert_eq!(expected, actual);
}

#[test]
fn renamed_variant_enum_roundtrip() {
    let expected = RenamedVariantEnum::default();
    let actual =
        RenamedVariantEnum::from_value(RenamedVariantEnum::default().into_test_value()).unwrap();
    assert_eq!(expected, actual);

    let expected = RenamedVariantEnum::value();
    let actual = RenamedVariantEnum::from_value(RenamedVariantEnum::value())
        .unwrap()
        .into_test_value();
    assert_eq!(expected, actual);
}

#[derive(IntoValue, FromValue, Default, Debug, PartialEq)]
struct DefaultFieldStruct {
    #[nu_value(default)]
    field: String,
    #[nu_value(rename = "renamed", default)]
    field_two: String,
}

#[test]
fn default_field_struct_from_value() {
    let populated = DefaultFieldStruct {
        field: "hello".into(),
        field_two: "world".into(),
    };
    let populated_record = Value::test_record(record! {
        "field" => Value::test_string("hello"),
        "renamed" => Value::test_string("world"),
    });
    let actual = DefaultFieldStruct::from_value(populated_record).unwrap();
    assert_eq!(populated, actual);

    let default = DefaultFieldStruct::default();
    let default_record = Value::test_record(Record::new());
    let actual = DefaultFieldStruct::from_value(default_record).unwrap();
    assert_eq!(default, actual);
}
