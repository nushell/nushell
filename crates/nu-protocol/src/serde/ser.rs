use super::error::Error;
use crate::{Record, Span, Value};
use serde::Serialize;
use serde::ser::{
    SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple,
    SerializeTupleStruct, SerializeTupleVariant,
};

pub(crate) struct ValueSerializer {
    pub span: Span,
}

impl<'a> serde::Serializer for &'a ValueSerializer {
    type Ok = Value;
    type Error = Error;

    type SerializeSeq = SeqSerializer<'a>;
    type SerializeTuple = SeqSerializer<'a>;
    type SerializeTupleStruct = SeqSerializer<'a>;
    type SerializeTupleVariant = TupleVariantSerializer<'a>;

    type SerializeMap = MapSerializer<'a>;
    type SerializeStruct = StructSerializer<'a>;
    type SerializeStructVariant = StructVariantSerializer<'a>;

    fn serialize_bool(self, v: bool) -> Result<Value, Error> {
        Ok(Value::bool(v, self.span))
    }

    fn serialize_i8(self, v: i8) -> Result<Value, Error> {
        Ok(Value::int(v.into(), self.span))
    }

    fn serialize_i16(self, v: i16) -> Result<Value, Error> {
        Ok(Value::int(v.into(), self.span))
    }

    fn serialize_i32(self, v: i32) -> Result<Value, Error> {
        Ok(Value::int(v.into(), self.span))
    }

    fn serialize_i64(self, v: i64) -> Result<Value, Error> {
        Ok(Value::int(v, self.span))
    }

    fn serialize_u8(self, v: u8) -> Result<Value, Error> {
        Ok(Value::int(v.into(), self.span))
    }

    fn serialize_u16(self, v: u16) -> Result<Value, Error> {
        Ok(Value::int(v.into(), self.span))
    }

    fn serialize_u32(self, v: u32) -> Result<Value, Error> {
        Ok(Value::int(v.into(), self.span))
    }

    fn serialize_u64(self, v: u64) -> Result<Value, Error> {
        i64::try_from(v)
            .map(|i| Value::int(i, self.span))
            .map_err(|_| Error::new(format!("u64 value {v} exceeds i64::MAX")))
    }

    fn serialize_f32(self, v: f32) -> Result<Value, Error> {
        Ok(Value::float(v.into(), self.span))
    }

    fn serialize_f64(self, v: f64) -> Result<Value, Error> {
        Ok(Value::float(v, self.span))
    }

    fn serialize_char(self, v: char) -> Result<Value, Error> {
        Ok(Value::string(v, self.span))
    }

    fn serialize_str(self, v: &str) -> Result<Value, Error> {
        Ok(Value::string(v, self.span))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Value, Error> {
        Ok(Value::binary(v, self.span))
    }

    fn serialize_none(self) -> Result<Value, Error> {
        Ok(Value::nothing(self.span))
    }

    fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Result<Value, Error> {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Value, Error> {
        Ok(Value::nothing(self.span))
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Value, Error> {
        Ok(Value::nothing(self.span))
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Value, Error> {
        Ok(Value::string(variant, self.span))
    }

    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Value, Error> {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Value, Error> {
        let inner = value.serialize(self)?;
        let mut record = Record::with_capacity(1);
        record.push(variant.to_owned(), inner);
        Ok(Value::record(record, self.span))
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<SeqSerializer<'a>, Error> {
        Ok(SeqSerializer {
            ser: self,
            vals: Vec::with_capacity(len.unwrap_or(0)),
        })
    }

    fn serialize_tuple(self, len: usize) -> Result<SeqSerializer<'a>, Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<SeqSerializer<'a>, Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<TupleVariantSerializer<'a>, Error> {
        Ok(TupleVariantSerializer {
            ser: self,
            variant: variant.to_owned(),
            vals: Vec::with_capacity(len),
        })
    }

    fn serialize_map(self, len: Option<usize>) -> Result<MapSerializer<'a>, Error> {
        Ok(MapSerializer {
            ser: self,
            record: Record::with_capacity(len.unwrap_or(0)),
            pending_key: None,
        })
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<StructSerializer<'a>, Error> {
        Ok(StructSerializer {
            ser: self,
            record: Record::with_capacity(len),
        })
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<StructVariantSerializer<'a>, Error> {
        Ok(StructVariantSerializer {
            ser: self,
            variant: variant.to_owned(),
            record: Record::with_capacity(len),
        })
    }
}

pub(crate) struct SeqSerializer<'a> {
    ser: &'a ValueSerializer,
    vals: Vec<Value>,
}

impl SerializeSeq for SeqSerializer<'_> {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        self.vals.push(value.serialize(self.ser)?);
        Ok(())
    }

    fn end(self) -> Result<Value, Error> {
        Ok(Value::list(self.vals, self.ser.span))
    }
}

impl SerializeTuple for SeqSerializer<'_> {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Value, Error> {
        SerializeSeq::end(self)
    }
}

impl SerializeTupleStruct for SeqSerializer<'_> {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Value, Error> {
        SerializeSeq::end(self)
    }
}

pub(crate) struct TupleVariantSerializer<'a> {
    ser: &'a ValueSerializer,
    variant: String,
    vals: Vec<Value>,
}

impl SerializeTupleVariant for TupleVariantSerializer<'_> {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        self.vals.push(value.serialize(self.ser)?);
        Ok(())
    }

    fn end(self) -> Result<Value, Error> {
        let inner = Value::list(self.vals, self.ser.span);
        let mut record = Record::with_capacity(1);
        record.push(self.variant, inner);
        Ok(Value::record(record, self.ser.span))
    }
}

pub(crate) struct MapSerializer<'a> {
    ser: &'a ValueSerializer,
    record: Record,
    pending_key: Option<String>,
}

impl SerializeMap for MapSerializer<'_> {
    type Ok = Value;
    type Error = Error;

    fn serialize_key<T: Serialize + ?Sized>(&mut self, key: &T) -> Result<(), Error> {
        let key_val = key.serialize(self.ser)?;
        let key_str = match key_val {
            Value::String { val, .. } => val,
            other => {
                return Err(Error::new(format!(
                    "map key must serialize to string, got {}",
                    other.get_type()
                )));
            }
        };
        self.pending_key = Some(key_str);
        Ok(())
    }

    fn serialize_value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        let key = self
            .pending_key
            .take()
            .ok_or_else(|| Error::new("serialize_value called before serialize_key"))?;
        self.record.push(key, value.serialize(self.ser)?);
        Ok(())
    }

    fn end(self) -> Result<Value, Error> {
        Ok(Value::record(self.record, self.ser.span))
    }
}

pub(crate) struct StructSerializer<'a> {
    ser: &'a ValueSerializer,
    record: Record,
}

impl SerializeStruct for StructSerializer<'_> {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Error> {
        self.record.push(key.to_owned(), value.serialize(self.ser)?);
        Ok(())
    }

    fn end(self) -> Result<Value, Error> {
        Ok(Value::record(self.record, self.ser.span))
    }
}

pub(crate) struct StructVariantSerializer<'a> {
    ser: &'a ValueSerializer,
    variant: String,
    record: Record,
}

impl SerializeStructVariant for StructVariantSerializer<'_> {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Error> {
        self.record.push(key.to_owned(), value.serialize(self.ser)?);
        Ok(())
    }

    fn end(self) -> Result<Value, Error> {
        let inner = Value::record(self.record, self.ser.span);
        let mut outer = Record::with_capacity(1);
        outer.push(self.variant, inner);
        Ok(Value::record(outer, self.ser.span))
    }
}
