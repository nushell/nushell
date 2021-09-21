//! Convert any value implementing `serde::Serialize` into a
//! `nu_protocol::Value` using `nu_serde::to_value`. Compare the below manual
//! implemeentation and the one using `nu_serde`.
//!
//! ```
//! use nu_protocol::{Dictionary, Primitive, UntaggedValue, Value};
//! use nu_source::Tag;
//! use serde::Serialize;
//!
//! #[derive(Serialize)]
//! struct MyStruct {
//!     index: usize,
//!     name: String,
//! }
//!
//! fn manual(s: MyStruct, tag: Tag) -> Value {
//!     let mut dict = Dictionary::default();
//!     dict.insert(
//!         "index".into(),
//!         Value {
//!             value: UntaggedValue::Primitive(Primitive::Int(s.index as i64)),
//!             tag: tag.clone(),
//!         },
//!     );
//!     dict.insert(
//!         "name".into(),
//!         Value {
//!             value: UntaggedValue::Primitive(Primitive::String(s.name)),
//!             tag: tag.clone(),
//!         },
//!     );
//!
//!     Value {
//!         value: UntaggedValue::Row(dict),
//!         tag,
//!     }
//! }
//!
//! fn auto(s: &MyStruct, tag: Tag) -> Value {
//!     nu_serde::to_value(s, tag).unwrap()
//! }
//! ```

use bigdecimal::{BigDecimal, FromPrimitive};
use nu_protocol::value::dict::Dictionary;
use nu_protocol::{Primitive, ReturnSuccess, ReturnValue, UntaggedValue, Value};
use nu_source::Tag;
use serde::Serialize;

#[cfg(test)]
mod test;

#[derive(Debug, thiserror::Error, Serialize)]
pub enum Error {
    #[error("{0}")]
    SerdeCustom(String),

    #[error("Expceted serializer to provide map implementation with a key before value")]
    MapValueLackedKey,

    #[error("Expceted map key to be string, found {0:?}")]
    InvalidMapKey(Value),

    #[error("Failed to convert f32 value into BigDecimal")]
    F32BigDecimalError(f32),

    #[error("Failed to convert f64 value into BigDecimal")]
    F64BigDecimalError(f64),
}

impl serde::ser::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        Self::SerdeCustom(msg.to_string())
    }
}

/// Convert any value into a `nu_protocol::Value`
pub fn to_value<T>(value: T, tag: impl Into<Tag>) -> Result<Value, Error>
where
    T: Serialize,
{
    value.serialize(&Serializer { tag: tag.into() })
}

/// Convenience function that takes an iterator over values and turns them into
/// a `Vec<ReturnValue>` (all successful). This is necessary for the return
/// signatures of most functions in the `nu_plugin::Plugin` trait.
pub fn to_success_return_values<T>(
    values: impl IntoIterator<Item = T>,
    tag: impl Into<Tag>,
) -> Result<Vec<ReturnValue>, Error>
where
    T: Serialize,
{
    let tag = tag.into();

    let mut out_values = Vec::new();

    for value in values {
        let value = to_value(&value, &tag)?;

        out_values.push(ReturnValue::Ok(ReturnSuccess::Value(value)));
    }

    Ok(out_values)
}

struct Serializer {
    tag: Tag,
}

struct SeqSerializer<'a> {
    seq: Vec<Value>,
    serializer: &'a Serializer,
}

struct MapSerializer<'a> {
    dict: Dictionary,
    serializer: &'a Serializer,
    current_key: Option<String>,
}

impl Serializer {
    fn value(&self, untagged: UntaggedValue) -> Value {
        Value {
            value: untagged,
            tag: self.tag.clone(),
        }
    }
}

impl<'a> serde::ser::SerializeSeq for SeqSerializer<'a> {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.seq.push(value.serialize(self.serializer)?);

        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.serializer.value(UntaggedValue::Table(self.seq)))
    }
}

impl<'a> serde::ser::SerializeTuple for SeqSerializer<'a> {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.seq.push(value.serialize(self.serializer)?);

        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.serializer.value(UntaggedValue::Table(self.seq)))
    }
}

impl<'a> serde::ser::SerializeTupleStruct for SeqSerializer<'a> {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.seq.push(value.serialize(self.serializer)?);

        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.serializer.value(UntaggedValue::Table(self.seq)))
    }
}

impl<'a> serde::ser::SerializeTupleVariant for SeqSerializer<'a> {
    type Ok = Value;

    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.seq.push(value.serialize(self.serializer)?);

        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.serializer.value(UntaggedValue::Table(self.seq)))
    }
}

impl<'a> serde::ser::SerializeMap for MapSerializer<'a> {
    type Ok = Value;
    type Error = Error;

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        let key_value = key.serialize(self.serializer)?;

        let key = match key_value.value {
            UntaggedValue::Primitive(Primitive::String(s)) => s,
            _ => return Err(Error::InvalidMapKey(key_value)),
        };

        self.current_key = Some(key);

        Ok(())
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        let key = self.current_key.take().ok_or(Error::MapValueLackedKey)?;

        self.dict.insert(key, value.serialize(self.serializer)?);

        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.serializer.value(UntaggedValue::Row(self.dict)))
    }
}

impl<'a> serde::ser::SerializeStruct for MapSerializer<'a> {
    type Ok = Value;

    type Error = Error;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.dict
            .insert(key.to_owned(), value.serialize(self.serializer)?);

        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.serializer.value(UntaggedValue::Row(self.dict)))
    }
}

impl<'a> serde::ser::SerializeStructVariant for MapSerializer<'a> {
    type Ok = Value;

    type Error = Error;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.dict
            .insert(key.to_owned(), value.serialize(self.serializer)?);

        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.serializer.value(UntaggedValue::Row(self.dict)))
    }
}

impl<'a> SeqSerializer<'a> {
    fn new(serializer: &'a Serializer) -> Self {
        Self {
            seq: Vec::new(),
            serializer,
        }
    }
}

impl<'a> MapSerializer<'a> {
    fn new(serializer: &'a Serializer) -> Self {
        Self {
            dict: Dictionary::default(),
            current_key: None,
            serializer,
        }
    }
}

impl<'a> serde::Serializer for &'a Serializer {
    type Ok = Value;
    type Error = Error;

    type SerializeSeq = SeqSerializer<'a>;
    type SerializeTuple = SeqSerializer<'a>;
    type SerializeTupleStruct = SeqSerializer<'a>;
    type SerializeTupleVariant = SeqSerializer<'a>;
    type SerializeMap = MapSerializer<'a>;
    type SerializeStruct = MapSerializer<'a>;
    type SerializeStructVariant = MapSerializer<'a>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        Ok(self.value(UntaggedValue::Primitive(Primitive::Boolean(v))))
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        Ok(self.value(UntaggedValue::Primitive(Primitive::Int(v as i64))))
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        Ok(self.value(UntaggedValue::Primitive(Primitive::Int(v as i64))))
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        Ok(self.value(UntaggedValue::Primitive(Primitive::Int(v as i64))))
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        Ok(self.value(UntaggedValue::Primitive(Primitive::Int(v as i64))))
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        Ok(self.value(UntaggedValue::Primitive(Primitive::Int(v as i64))))
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        Ok(self.value(UntaggedValue::Primitive(Primitive::Int(v as i64))))
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        Ok(self.value(UntaggedValue::Primitive(Primitive::Int(v as i64))))
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        Ok(self.value(UntaggedValue::Primitive(Primitive::BigInt(v.into()))))
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        Ok(self.value(UntaggedValue::Primitive(Primitive::Decimal(
            BigDecimal::from_f32(v).ok_or(Error::F32BigDecimalError(v))?,
        ))))
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        Ok(self.value(UntaggedValue::Primitive(Primitive::Decimal(
            BigDecimal::from_f64(v).ok_or(Error::F64BigDecimalError(v))?,
        ))))
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        Ok(self.value(UntaggedValue::Primitive(Primitive::String(v.into()))))
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        Ok(self.value(UntaggedValue::Primitive(Primitive::String(v.into()))))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        Ok(self.value(UntaggedValue::Primitive(Primitive::Binary(v.into()))))
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.value(UntaggedValue::Primitive(Primitive::Nothing)))
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        // TODO: is this OK?
        Ok(self.value(UntaggedValue::Primitive(Primitive::Nothing)))
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        // TODO: is this OK?
        Ok(self.value(UntaggedValue::Primitive(Primitive::Nothing)))
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        // TODO: is this OK?
        Ok(self.value(UntaggedValue::Primitive(Primitive::Nothing)))
    }

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        value.serialize(self)
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(SeqSerializer::new(self))
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Ok(SeqSerializer::new(self))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Ok(SeqSerializer::new(self))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Ok(SeqSerializer::new(self))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(MapSerializer::new(self))
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(MapSerializer::new(self))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Ok(MapSerializer::new(self))
    }
}
