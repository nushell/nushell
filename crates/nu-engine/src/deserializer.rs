use log::trace;
use nu_errors::{CoerceInto, ShellError};
use nu_protocol::{
    hir::CapturedBlock, CallInfo, ColumnPath, Primitive, RangeInclusion, ShellTypeName,
    UntaggedValue, Value,
};
use nu_source::{HasSpan, Spanned, SpannedItem, Tagged, TaggedItem};
use nu_value_ext::ValueExt;
use serde::de;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Copy, Clone, Deserialize, Serialize)]
pub struct NumericRange {
    pub from: (Option<Spanned<u64>>, RangeInclusion),
    pub to: (Option<Spanned<u64>>, RangeInclusion),
}

impl NumericRange {
    pub fn min(self) -> u64 {
        match self.from.1 {
            RangeInclusion::Inclusive => self.from.0.map(|from| *from).unwrap_or(0),
            RangeInclusion::Exclusive => {
                self.from.0.map(|from| *from).unwrap_or(0).saturating_add(1)
            }
        }
    }

    pub fn max(self) -> u64 {
        match self.to.1 {
            RangeInclusion::Inclusive => self.to.0.map(|to| *to).unwrap_or(u64::MAX),
            RangeInclusion::Exclusive => self
                .to
                .0
                .map(|to| *to)
                .unwrap_or(u64::MAX)
                .saturating_sub(1),
        }
    }
}

#[derive(Debug)]
pub struct DeserializerItem<'de> {
    key_struct_field: Option<(String, &'de str)>,
    val: Value,
}

pub struct ConfigDeserializer<'de> {
    call: CallInfo,
    stack: Vec<DeserializerItem<'de>>,
    saw_root: bool,
    position: usize,
}

impl<'de> ConfigDeserializer<'de> {
    pub fn from_call_info(call: CallInfo) -> ConfigDeserializer<'de> {
        ConfigDeserializer {
            call,
            stack: vec![],
            saw_root: false,
            position: 0,
        }
    }

    pub fn push_val(&mut self, val: Value) {
        self.stack.push(DeserializerItem {
            key_struct_field: None,
            val,
        });
    }

    pub fn push(&mut self, name: &'static str) -> Result<(), ShellError> {
        let value: Option<Value> = if name == "rest" {
            let positional = self.call.args.slice_from(self.position);
            self.position += positional.len();
            Some(UntaggedValue::Table(positional).into_untagged_value()) // TODO: correct tag
        } else if self.call.args.has(name) {
            self.call.args.get(name).cloned()
        } else {
            let position = self.position;
            self.position += 1;
            self.call.args.nth(position).cloned()
        };

        trace!("pushing {:?}", value);

        self.stack.push(DeserializerItem {
            key_struct_field: Some((name.to_string(), name)),
            val: value.unwrap_or_else(|| UntaggedValue::nothing().into_value(&self.call.name_tag)),
        });

        Ok(())
    }

    pub fn top(&mut self) -> &DeserializerItem {
        let value = self.stack.last();
        trace!("inspecting top value :: {:?}", value);
        value.expect("Can't get top element of an empty stack")
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
        unimplemented!("deserialize_any")
    }
    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value = self.pop();
        trace!("Extracting {:?} for bool", value.val);

        match &value.val {
            Value {
                value: UntaggedValue::Primitive(Primitive::Boolean(b)),
                ..
            } => visitor.visit_bool(*b),
            Value {
                value: UntaggedValue::Primitive(Primitive::Nothing),
                ..
            } => visitor.visit_bool(false),
            other => Err(ShellError::type_error(
                "Boolean",
                other.type_name().spanned(other.span()),
            )),
        }
    }
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

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value = self.top();
        let name = std::any::type_name::<V::Value>();
        trace!("<Option> Extracting {:?} for Option<{}>", value, name);
        match &value.val.value {
            UntaggedValue::Primitive(Primitive::Nothing) => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
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
    fn deserialize_seq<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value = self.pop();
        trace!("<Vec> Extracting {:?} for vec", value.val);

        match value.val.into_parts() {
            (UntaggedValue::Table(items), _) => {
                let de = SeqDeserializer::new(&mut self, items.into_iter());
                visitor.visit_seq(de)
            }
            (other, tag) => Err(ShellError::type_error(
                "Vec",
                other.type_name().spanned(tag),
            )),
        }
    }
    fn deserialize_tuple<V>(mut self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value = self.pop();
        trace!(
            "<Tuple> Extracting {:?} for tuple with {} elements",
            value.val,
            len
        );

        match value.val.into_parts() {
            (UntaggedValue::Table(items), _) => {
                let de = SeqDeserializer::new(&mut self, items.into_iter());
                visitor.visit_seq(de)
            }
            (other, tag) => Err(ShellError::type_error(
                "Tuple",
                other.type_name().spanned(tag),
            )),
        }
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
        fn visit<'de, T, V>(
            val: T,
            name: &'static str,
            fields: &'static [&'static str],
            visitor: V,
        ) -> Result<V::Value, ShellError>
        where
            T: serde::Serialize,
            V: Visitor<'de>,
        {
            let json = serde_json::to_string(&val)?;
            let json_cursor = std::io::Cursor::new(json.into_bytes());
            let mut json_de = serde_json::Deserializer::from_reader(json_cursor);
            let r = json_de.deserialize_struct(name, fields, visitor)?;
            Ok(r)
        }

        trace!(
            "deserializing struct {:?} {:?} (saw_root={} stack={:?})",
            name,
            fields,
            self.saw_root,
            self.stack
        );

        if !self.saw_root {
            self.saw_root = true;
            return visitor.visit_seq(StructDeserializer::new(&mut self, fields));
        }

        let value = self.pop();

        let type_name = std::any::type_name::<V::Value>();
        let tagged_val_name = std::any::type_name::<Value>();

        trace!(
            "name={} type_name={} tagged_val_name={}",
            name,
            type_name,
            tagged_val_name
        );

        if type_name == tagged_val_name {
            return visit::<Value, _>(value.val, name, fields, visitor);
        }

        if name == "CapturedBlock" {
            let block = match value.val {
                Value {
                    value: UntaggedValue::Block(block),
                    ..
                } => block,
                other => {
                    return Err(ShellError::type_error(
                        "Block",
                        other.type_name().spanned(other.span()),
                    ))
                }
            };
            return visit::<CapturedBlock, _>(*block, name, fields, visitor);
        }

        if name == "ColumnPath" {
            let path = match value.val {
                Value {
                    value: UntaggedValue::Primitive(Primitive::ColumnPath(path)),
                    ..
                } => path,
                other => {
                    return Err(ShellError::type_error(
                        "column path",
                        other.type_name().spanned(other.span()),
                    ))
                }
            };

            return visit::<ColumnPath, _>(path, name, fields, visitor);
        }

        trace!("Extracting {:?} for {:?}", value.val, type_name);

        let tag = value.val.tag();
        match value.val {
            Value {
                value: UntaggedValue::Primitive(Primitive::Boolean(b)),
                ..
            } => visit::<Tagged<bool>, _>(b.tagged(tag), name, fields, visitor),
            Value {
                value: UntaggedValue::Primitive(Primitive::Nothing),
                ..
            } => visit::<Tagged<bool>, _>(false.tagged(tag), name, fields, visitor),
            Value {
                value: UntaggedValue::Primitive(Primitive::FilePath(p)),
                ..
            } => visit::<Tagged<PathBuf>, _>(p.tagged(tag), name, fields, visitor),
            Value {
                value: UntaggedValue::Primitive(Primitive::Int(int)),
                ..
            } => {
                let i: i64 = int.tagged(value.val.tag).coerce_into("converting to i64")?;
                visit::<Tagged<i64>, _>(i.tagged(tag), name, fields, visitor)
            }
            Value {
                value: UntaggedValue::Primitive(Primitive::Duration(big_int)),
                ..
            } => {
                let u_int: u64 = big_int
                    .tagged(value.val.tag)
                    .coerce_into("converting to u64")?;
                visit::<Tagged<u64>, _>(u_int.tagged(tag), name, fields, visitor)
            }
            Value {
                value: UntaggedValue::Primitive(Primitive::Decimal(decimal)),
                ..
            } => {
                let i: f64 = decimal
                    .tagged(value.val.tag)
                    .coerce_into("converting to f64")?;
                visit::<Tagged<f64>, _>(i.tagged(tag), name, fields, visitor)
            }
            Value {
                value: UntaggedValue::Primitive(Primitive::String(string)),
                ..
            } => visit::<Tagged<String>, _>(string.tagged(tag), name, fields, visitor),
            Value {
                value: UntaggedValue::Primitive(Primitive::Range(range)),
                ..
            } => {
                let (left, left_inclusion) = range.from;
                let (right, right_inclusion) = range.to;
                let left_span = left.span;
                let right_span = right.span;

                let left = match left.item {
                    Primitive::Nothing => None,
                    _ => Some(left.as_u64(left_span)?),
                };
                let right = match right.item {
                    Primitive::Nothing => None,
                    _ => Some(right.as_u64(right_span)?),
                };

                let numeric_range = NumericRange {
                    from: (left.map(|left| left.spanned(left_span)), left_inclusion),
                    to: (
                        right.map(|right| right.spanned(right_span)),
                        right_inclusion,
                    ),
                };

                visit::<Tagged<NumericRange>, _>(numeric_range.tagged(tag), name, fields, visitor)
            }

            other => Err(ShellError::type_error(
                name,
                other.type_name().spanned(other.span()),
            )),
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

struct SeqDeserializer<'a, 'de: 'a, I: Iterator<Item = Value>> {
    de: &'a mut ConfigDeserializer<'de>,
    vals: I,
}

impl<'a, 'de: 'a, I: Iterator<Item = Value>> SeqDeserializer<'a, 'de, I> {
    fn new(de: &'a mut ConfigDeserializer<'de>, vals: I) -> Self {
        SeqDeserializer { de, vals }
    }
}

impl<'a, 'de: 'a, I: Iterator<Item = Value>> de::SeqAccess<'de> for SeqDeserializer<'a, 'de, I> {
    type Error = ShellError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        let next = if let Some(next) = self.vals.next() {
            next
        } else {
            return Ok(None);
        };

        self.de.push_val(next);
        seed.deserialize(&mut *self.de).map(Some)
    }

    fn size_hint(&self) -> Option<usize> {
        self.vals.size_hint().1
    }
}

struct StructDeserializer<'a, 'de: 'a> {
    de: &'a mut ConfigDeserializer<'de>,
    fields: &'static [&'static str],
}

impl<'a, 'de: 'a> StructDeserializer<'a, 'de> {
    fn new(de: &'a mut ConfigDeserializer<'de>, fields: &'static [&'static str]) -> Self {
        StructDeserializer { de, fields }
    }
}

impl<'a, 'de: 'a> de::SeqAccess<'de> for StructDeserializer<'a, 'de> {
    type Error = ShellError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        if self.fields.is_empty() {
            return Ok(None);
        }

        trace!("Processing {}", self.fields[0]);

        self.de.push(self.fields[0])?;
        self.fields = &self.fields[1..];
        seed.deserialize(&mut *self.de).map(Some)
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.fields.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::any::type_name;
    #[test]
    fn check_type_name_properties() {
        // This ensures that certain properties for the
        // std::any::type_name function hold, that
        // this code relies on. The type_name docs explicitly
        // mention that the actual format of the output
        // is unspecified and change is likely.
        // This test makes sure that such change is detected
        // by this test failing, and not things silently breaking.
        // Specifically, we rely on this behavior further above
        // in the file for the Value special case parsing.
        let tuple = type_name::<()>();
        let tagged_tuple = type_name::<Tagged<()>>();
        let tagged_value = type_name::<Value>();
        assert_ne!(tuple, tagged_tuple);
        assert_ne!(tuple, tagged_value);
        assert_ne!(tagged_tuple, tagged_value);
    }
}
