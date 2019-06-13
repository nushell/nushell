use crate::object::base::OF64;
use crate::prelude::*;

use ordered_float::OrderedFloat;
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

struct OF64Visitor;

impl Visitor<'_> for OF64Visitor {
    type Value = OF64;

    fn visit_f64<E>(self, value: f64) -> Result<OF64, E> {
        Ok(OF64::new(OrderedFloat(value)))
    }

    fn visit_f32<E>(self, value: f32) -> Result<OF64, E> {
        Ok(OF64::new(OrderedFloat(value as f64)))
    }

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a float")
    }
}

impl<'de> Deserialize<'de> for OF64 {
    fn deserialize<D>(deserializer: D) -> Result<OF64, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_f64(OF64Visitor)
    }
}

impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Value::Primitive(p) => p.serialize(serializer),
            Value::Object(o) => o.serialize(serializer),
            Value::List(l) => l.serialize(serializer),
            Value::Block(b) => b.serialize(serializer),
            Value::Error(e) => e.serialize(serializer),
            Value::Filesystem => "".serialize(serializer),
        }
    }
}

struct ValueVisitor;

impl<'de> Visitor<'de> for ValueVisitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a shell value")
    }

    fn visit_i32<E>(self, value: i32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::int(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::int(value))
    }

    fn visit_u32<E>(self, value: u32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::int(value))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        // TODO: Handle overflow better
        Ok(Value::int(value as i64))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::string(value))
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::boolean(value))
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(ValueVisitor)
    }
}
