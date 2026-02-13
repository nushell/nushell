use super::error::Error;
use crate::Value;
use serde::Deserializer as _;
use serde::de::{
    self, DeserializeSeed, EnumAccess, IntoDeserializer, MapAccess, SeqAccess, VariantAccess,
    Visitor,
};

pub(crate) struct ValueDeserializer<'de> {
    pub value: &'de Value,
}

impl<'de> de::Deserializer<'de> for ValueDeserializer<'de> {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.value {
            Value::Bool { val, .. } => visitor.visit_bool(*val),
            Value::Int { val, .. } => visitor.visit_i64(*val),
            Value::Float { val, .. } => visitor.visit_f64(*val),
            Value::String { val, .. } => visitor.visit_str(val),
            Value::Binary { val, .. } => visitor.visit_bytes(val),
            Value::Nothing { .. } => visitor.visit_unit(),
            Value::List { vals, .. } => visitor.visit_seq(NuSeqAccess::new(vals)),
            Value::Record { val, .. } => visitor.visit_map(NuMapAccess::new(val)),
            other => Err(Error::new(format!(
                "unsupported Value type for deserialization: {}",
                other.get_type()
            ))),
        }
    }

    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.value {
            Value::Bool { val, .. } => visitor.visit_bool(*val),
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_i8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.value {
            Value::Int { val, .. } => visitor.visit_i8(*val as i8),
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_i16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.value {
            Value::Int { val, .. } => visitor.visit_i16(*val as i16),
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_i32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.value {
            Value::Int { val, .. } => visitor.visit_i32(*val as i32),
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_i64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.value {
            Value::Int { val, .. } => visitor.visit_i64(*val),
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_u8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.value {
            Value::Int { val, .. } => visitor.visit_u8(*val as u8),
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_u16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.value {
            Value::Int { val, .. } => visitor.visit_u16(*val as u16),
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_u32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.value {
            Value::Int { val, .. } => visitor.visit_u32(*val as u32),
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_u64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.value {
            Value::Int { val, .. } if *val >= 0 => visitor.visit_u64(*val as u64),
            Value::Int { val, .. } => Err(Error::new(format!(
                "cannot deserialize negative value {val} as u64"
            ))),
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_f32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.value {
            Value::Float { val, .. } => visitor.visit_f32(*val as f32),
            Value::Int { val, .. } => visitor.visit_f32(*val as f32),
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_f64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.value {
            Value::Float { val, .. } => visitor.visit_f64(*val),
            Value::Int { val, .. } => visitor.visit_f64(*val as f64),
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_char<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.value {
            Value::String { val, .. } => {
                let mut chars = val.chars();
                match (chars.next(), chars.next()) {
                    (Some(c), None) => visitor.visit_char(c),
                    _ => Err(Error::new(format!(
                        "expected single char, got string of length {}",
                        val.len()
                    ))),
                }
            }
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.value {
            Value::String { val, .. } => visitor.visit_str(val),
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.value {
            Value::Binary { val, .. } => visitor.visit_bytes(val),
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.value {
            Value::Nothing { .. } => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.value {
            Value::Nothing { .. } => visitor.visit_unit(),
            _ => Err(Error::new(format!(
                "expected nothing for unit, got {}",
                self.value.get_type()
            ))),
        }
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.value {
            Value::List { vals, .. } => visitor.visit_seq(NuSeqAccess::new(vals)),
            _ => Err(Error::new(format!(
                "expected list, got {}",
                self.value.get_type()
            ))),
        }
    }

    fn deserialize_tuple<V: Visitor<'de>>(
        self,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.value {
            Value::Record { val, .. } => visitor.visit_map(NuMapAccess::new(val)),
            _ => Err(Error::new(format!(
                "expected record, got {}",
                self.value.get_type()
            ))),
        }
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        match self.value {
            // Unit variant: serialized as plain string "VariantName"
            Value::String { val, .. } => visitor.visit_enum(NuEnumAccess {
                variant: val.as_str(),
                content: None,
            }),
            // Non-unit variant: serialized as { "VariantName": content }
            Value::Record { val, .. } => {
                let mut iter = val.iter();
                let (Some((key, value)), None) = (iter.next(), iter.next()) else {
                    return Err(Error::new(format!(
                        "expected single-key record for enum, got {} keys",
                        val.len()
                    )));
                };
                visitor.visit_enum(NuEnumAccess {
                    variant: key.as_str(),
                    content: Some(value),
                })
            }
            _ => Err(Error::new(format!(
                "expected string or single-key record for enum, got {}",
                self.value.get_type()
            ))),
        }
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_unit()
    }
}

struct NuSeqAccess<'de> {
    iter: std::slice::Iter<'de, Value>,
}

impl<'de> NuSeqAccess<'de> {
    fn new(vals: &'de [Value]) -> Self {
        NuSeqAccess { iter: vals.iter() }
    }
}

impl<'de> SeqAccess<'de> for NuSeqAccess<'de> {
    type Error = Error;

    fn next_element_seed<T: DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, Error> {
        match self.iter.next() {
            Some(value) => seed.deserialize(ValueDeserializer { value }).map(Some),
            None => Ok(None),
        }
    }
}

struct NuMapAccess<'de> {
    iter: Box<dyn Iterator<Item = (&'de String, &'de Value)> + 'de>,
    pending_value: Option<&'de Value>,
}

impl<'de> NuMapAccess<'de> {
    fn new(record: &'de crate::Record) -> Self {
        NuMapAccess {
            iter: Box::new(record.iter()),
            pending_value: None,
        }
    }
}

impl<'de> MapAccess<'de> for NuMapAccess<'de> {
    type Error = Error;

    fn next_key_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Error> {
        match self.iter.next() {
            Some((key, value)) => {
                self.pending_value = Some(value);
                seed.deserialize(key.as_str().into_deserializer()).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value, Error> {
        let value = self
            .pending_value
            .take()
            .ok_or_else(|| Error::new("next_value_seed called before next_key_seed"))?;
        seed.deserialize(ValueDeserializer { value })
    }
}

struct NuEnumAccess<'de> {
    variant: &'de str,
    content: Option<&'de Value>,
}

impl<'de> EnumAccess<'de> for NuEnumAccess<'de> {
    type Error = Error;
    type Variant = NuVariantAccess<'de>;

    fn variant_seed<V: DeserializeSeed<'de>>(
        self,
        seed: V,
    ) -> Result<(V::Value, Self::Variant), Error> {
        let variant = seed.deserialize(self.variant.into_deserializer())?;
        Ok((
            variant,
            NuVariantAccess {
                content: self.content,
            },
        ))
    }
}

struct NuVariantAccess<'de> {
    content: Option<&'de Value>,
}

impl<'de> VariantAccess<'de> for NuVariantAccess<'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Error> {
        Ok(())
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value, Error> {
        let value = self
            .content
            .ok_or_else(|| Error::new("expected content for newtype variant"))?;
        seed.deserialize(ValueDeserializer { value })
    }

    fn tuple_variant<V: Visitor<'de>>(self, _len: usize, visitor: V) -> Result<V::Value, Error> {
        let value = self
            .content
            .ok_or_else(|| Error::new("expected content for tuple variant"))?;
        ValueDeserializer { value }.deserialize_seq(visitor)
    }

    fn struct_variant<V: Visitor<'de>>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        let value = self
            .content
            .ok_or_else(|| Error::new("expected content for struct variant"))?;
        ValueDeserializer { value }.deserialize_map(visitor)
    }
}
