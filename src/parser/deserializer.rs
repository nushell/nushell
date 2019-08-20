use crate::prelude::*;
use log::trace;
use serde::{de, forward_to_deserialize_any};

#[derive(Debug)]
pub struct DeserializerItem<'de> {
    key: String,
    struct_field: &'de str,
    val: Tagged<Value>,
}

pub struct ConfigDeserializer<'de> {
    call: CallInfo,
    stack: Vec<DeserializerItem<'de>>,
    saw_root: bool,
    position: usize,
}

impl ConfigDeserializer<'de> {
    pub fn from_call_info(call: CallInfo) -> ConfigDeserializer<'de> {
        ConfigDeserializer {
            call,
            stack: vec![],
            saw_root: false,
            position: 0,
        }
    }

    pub fn push(&mut self, name: &'static str) -> Result<(), ShellError> {
        let value: Option<Tagged<Value>> = if name == "rest" {
            let positional = self.call.args.slice_from(self.position);
            self.position += positional.len();
            Some(Value::List(positional).tagged_unknown()) // TODO: correct span
        } else {
            if self.call.args.has(name) {
                self.call.args.get(name).map(|x| x.clone())
            } else {
                let position = self.position;
                self.position += 1;
                self.call.args.nth(position).map(|x| x.clone())
            }
        };

        trace!("pushing {:?}", value);

        self.stack.push(DeserializerItem {
            key: name.to_string(),
            struct_field: name,
            val: value.unwrap_or_else(|| {
                Value::nothing().tagged(Tag::unknown_origin(self.call.name_span))
            }),
        });

        Ok(())
    }

    pub fn pop(&mut self) -> DeserializerItem {
        let value = self.stack.pop();
        trace!("popping value :: {:?}", value);
        value.expect("Can't pop an empty stack")
    }
}

use de::Visitor;

impl<'de, 'a> de::Deserializer<'de> for &'a mut ConfigDeserializer<'de> {
    type Error = ShellError;
    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value = self.pop();
        let name = std::intrinsics::type_name::<V::Value>();
        trace!("<Deserialize any> Extracting {:?}", name);

        V::Value::extract(&value.val)
    }

    forward_to_deserialize_any! { bool option seq }

    fn deserialize_i8<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!("deserialize_i8")
    }
    fn deserialize_i16<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!("deserialize_i16")
    }
    fn deserialize_i32<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!("deserialize_i32")
    }
    fn deserialize_i64<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!("deserialize_i64")
    }
    fn deserialize_u8<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!("deserialize_u8")
    }
    fn deserialize_u16<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!("deserialize_u16")
    }
    fn deserialize_u32<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!("deserialize_u32")
    }
    fn deserialize_u64<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!("deserialize_u64")
    }
    fn deserialize_f32<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!("deserialize_f32")
    }
    fn deserialize_f64<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!("deserialize_f64")
    }
    fn deserialize_char<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!("deserialize_char")
    }
    fn deserialize_str<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!("deserialize_str")
    }
    fn deserialize_string<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!("deserialize_string")
    }
    fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!("deserialize_bytes")
    }
    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!("deserialize_byte_buf")
    }

    fn deserialize_unit<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!("deserialize_unit")
    }
    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!("deserialize_unit_struct")
    }
    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!("deserialize_newtype_struct")
    }

    fn deserialize_tuple<V>(self, _len: usize, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!("deserialize_tuple")
    }
    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!("deserialize_tuple_struct")
    }
    fn deserialize_map<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!("deserialize_map")
    }
    fn deserialize_struct<V>(
        mut self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        trace!(
            "deserializing struct {:?} {:?} (stack={:?})",
            name,
            fields,
            self.stack
        );

        if self.saw_root {
            let value = self.pop();
            let name = std::intrinsics::type_name::<V::Value>();
            trace!("Extracting {:?} for {:?}", value.val, name);
            V::Value::extract(&value.val)
        } else {
            self.saw_root = true;
            visitor.visit_seq(StructDeserializer::new(&mut self, fields))
        }
    }
    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!("deserialize_enum")
    }
    fn deserialize_identifier<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!("deserialize_identifier")
    }
    fn deserialize_ignored_any<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!("deserialize_ignored_any")
    }
}

struct StructDeserializer<'a, 'de: 'a> {
    de: &'a mut ConfigDeserializer<'de>,
    fields: &'static [&'static str],
}

impl<'a, 'de: 'a> StructDeserializer<'a, 'de> {
    fn new(de: &'a mut ConfigDeserializer<'de>, fields: &'static [&'static str]) -> Self {
        StructDeserializer {
            de: de,
            fields: fields,
        }
    }
}

impl<'a, 'de: 'a> de::SeqAccess<'de> for StructDeserializer<'a, 'de> {
    type Error = ShellError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        if self.fields.len() == 0 {
            return Ok(None);
        }

        trace!("Processing {}", self.fields[0]);

        self.de.push(self.fields[0])?;
        self.fields = &self.fields[1..];
        seed.deserialize(&mut *self.de).map(Some)
    }

    fn size_hint(&self) -> Option<usize> {
        return Some(self.fields.len());
    }
}
