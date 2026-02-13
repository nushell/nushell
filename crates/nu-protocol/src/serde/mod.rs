//! Serde [`Serializer`](serde::Serializer) and [`Deserializer`](serde::Deserializer)
//! for converting between Rust types and nushell [`Value`].
//!
//! Provides [`to_value`] and [`from_value`] functions that use serde's traits
//! to convert directly to/from nushell Values without an intermediate
//! representation such as JSON.
//!
//! # Enum representation
//!
//! Serde's default externally-tagged enum format is used:
//! - Unit variants: `Value::String("VariantName")`
//! - Newtype variants: `Value::Record { "VariantName": inner }`
//! - Tuple variants: `Value::Record { "VariantName": [fields...] }`
//! - Struct variants: `Value::Record { "VariantName": { field: value, ... } }`

mod de;
pub mod error;
mod ser;

use crate::{Span, Value};
use serde::{Deserialize, Serialize};

/// Serialize any `T: Serialize` into a nushell [`Value`].
pub fn to_value<T: Serialize>(value: &T, span: Span) -> Result<Value, error::Error> {
    let serializer = ser::ValueSerializer { span };
    value.serialize(&serializer)
}

/// Deserialize any `T: Deserialize` from a nushell [`Value`].
pub fn from_value<'de, T: Deserialize<'de>>(value: &'de Value) -> Result<T, error::Error> {
    T::deserialize(de::ValueDeserializer { value })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Span, Value};
    use serde::{Deserialize, Serialize};

    fn span() -> Span {
        Span::test_data()
    }

    #[test]
    fn roundtrip_primitives() {
        let v = to_value(&true, span()).unwrap();
        assert_eq!(v, Value::bool(true, span()));
        assert!(from_value::<bool>(&v).unwrap());

        let v = to_value(&42i64, span()).unwrap();
        assert_eq!(v, Value::int(42, span()));
        assert_eq!(from_value::<i64>(&v).unwrap(), 42);

        let v = to_value(&1.2f64, span()).unwrap();
        assert_eq!(v, Value::float(1.2, span()));
        assert_eq!(from_value::<f64>(&v).unwrap(), 1.2);

        let v = to_value(&"hello", span()).unwrap();
        assert_eq!(v, Value::string("hello", span()));
        assert_eq!(from_value::<String>(&v).unwrap(), "hello");
    }

    #[test]
    fn roundtrip_option() {
        let v = to_value(&Some(42i64), span()).unwrap();
        assert_eq!(from_value::<Option<i64>>(&v).unwrap(), Some(42));

        let v = to_value(&None::<i64>, span()).unwrap();
        assert_eq!(v, Value::nothing(span()));
        assert_eq!(from_value::<Option<i64>>(&v).unwrap(), None);
    }

    #[test]
    fn roundtrip_vec() {
        let orig = vec![1i64, 2, 3];
        let v = to_value(&orig, span()).unwrap();
        assert_eq!(from_value::<Vec<i64>>(&v).unwrap(), orig);
    }

    #[test]
    fn roundtrip_struct() {
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct Point {
            x: i64,
            y: i64,
        }

        let orig = Point { x: 10, y: 20 };
        let v = to_value(&orig, span()).unwrap();

        let rec = v.as_record().unwrap();
        assert_eq!(rec.get("x").unwrap(), &Value::int(10, span()));
        assert_eq!(rec.get("y").unwrap(), &Value::int(20, span()));

        assert_eq!(from_value::<Point>(&v).unwrap(), orig);
    }

    #[test]
    fn roundtrip_enum_unit() {
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        enum Color {
            Red,
            Green,
            Blue,
        }

        let v = to_value(&Color::Red, span()).unwrap();
        assert_eq!(v, Value::string("Red", span()));
        assert_eq!(from_value::<Color>(&v).unwrap(), Color::Red);
    }

    #[test]
    fn roundtrip_enum_newtype() {
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        enum Shape {
            Circle(f64),
            Label(String),
        }

        let orig = Shape::Circle(5.0);
        let v = to_value(&orig, span()).unwrap();
        assert_eq!(from_value::<Shape>(&v).unwrap(), orig);

        let orig = Shape::Label("hi".into());
        let v = to_value(&orig, span()).unwrap();
        assert_eq!(from_value::<Shape>(&v).unwrap(), orig);
    }

    #[test]
    fn roundtrip_enum_tuple() {
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        enum Pair {
            IntPair(i64, i64),
        }

        let orig = Pair::IntPair(1, 2);
        let v = to_value(&orig, span()).unwrap();
        assert_eq!(from_value::<Pair>(&v).unwrap(), orig);
    }

    #[test]
    fn roundtrip_enum_struct() {
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        enum Node {
            Leaf { value: i64 },
            Branch { left: Box<Node>, right: Box<Node> },
        }

        let orig = Node::Branch {
            left: Box::new(Node::Leaf { value: 1 }),
            right: Box::new(Node::Leaf { value: 2 }),
        };
        let v = to_value(&orig, span()).unwrap();
        assert_eq!(from_value::<Node>(&v).unwrap(), orig);
    }

    #[test]
    fn roundtrip_nested_struct() {
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct Inner {
            name: String,
        }

        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct Outer {
            items: Vec<Inner>,
            count: u32,
        }

        let orig = Outer {
            items: vec![Inner { name: "a".into() }, Inner { name: "b".into() }],
            count: 2,
        };
        let v = to_value(&orig, span()).unwrap();
        assert_eq!(from_value::<Outer>(&v).unwrap(), orig);
    }

    #[test]
    fn roundtrip_u64_within_i64_range() {
        let v = to_value(&42u64, span()).unwrap();
        assert_eq!(from_value::<u64>(&v).unwrap(), 42);
    }

    #[test]
    fn u64_overflow_errors() {
        assert!(to_value(&u64::MAX, span()).is_err());
    }

    #[test]
    fn roundtrip_hashmap() {
        use std::collections::HashMap;
        let mut orig = HashMap::new();
        orig.insert("a".to_string(), 1i64);
        orig.insert("b".to_string(), 2);

        let v = to_value(&orig, span()).unwrap();
        let rt: HashMap<String, i64> = from_value(&v).unwrap();
        assert_eq!(rt, orig);
    }

    #[test]
    fn roundtrip_unit_struct() {
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct Marker;

        let v = to_value(&Marker, span()).unwrap();
        assert_eq!(v, Value::nothing(span()));
        assert_eq!(from_value::<Marker>(&v).unwrap(), Marker);
    }

    #[test]
    fn roundtrip_newtype_struct() {
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct Wrapper(i64);

        let orig = Wrapper(99);
        let v = to_value(&orig, span()).unwrap();
        assert_eq!(from_value::<Wrapper>(&v).unwrap(), orig);
    }

    #[test]
    fn roundtrip_tuple() {
        let orig = (1i64, "two".to_string(), 3.0f64);
        let v = to_value(&orig, span()).unwrap();
        assert_eq!(from_value::<(i64, String, f64)>(&v).unwrap(), orig);
    }

    #[test]
    fn roundtrip_complex_enum_with_box() {
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        enum Expr {
            Lit(i64),
            Add(Box<Expr>, Box<Expr>),
            Neg(Box<Expr>),
        }

        let orig = Expr::Neg(Box::new(Expr::Add(
            Box::new(Expr::Lit(1)),
            Box::new(Expr::Lit(2)),
        )));
        let v = to_value(&orig, span()).unwrap();
        assert_eq!(from_value::<Expr>(&v).unwrap(), orig);
    }

    #[test]
    fn roundtrip_skip_field_with_default() {
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct Config {
            name: String,
            #[serde(skip)]
            internal: i64,
        }

        let orig = Config {
            name: "test".into(),
            internal: 42,
        };
        let v = to_value(&orig, span()).unwrap();
        let rt: Config = from_value(&v).unwrap();
        assert_eq!(rt.name, "test");
        assert_eq!(rt.internal, 0);
    }
}
