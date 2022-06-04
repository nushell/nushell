#[cfg(not(feature = "preserve_order"))]
use std::collections::{btree_map, BTreeMap};

#[cfg(feature = "preserve_order")]
use linked_hash_map::{self, LinkedHashMap};

use std::fmt;
use std::io;
use std::str;
use std::vec;

use num_traits::NumCast;

use serde::de;
use serde::ser;

use crate::error::{Error, ErrorCode};

type Result<T, E = Error> = std::result::Result<T, E>;

/// Represents a key/value type.
#[cfg(not(feature = "preserve_order"))]
pub type Map<K, V> = BTreeMap<K, V>;
/// Represents a key/value type.
#[cfg(feature = "preserve_order")]
pub type Map<K, V> = LinkedHashMap<K, V>;

/// Represents the `IntoIter` type.
#[cfg(not(feature = "preserve_order"))]
pub type MapIntoIter<K, V> = btree_map::IntoIter<K, V>;
/// Represents the IntoIter type.
#[cfg(feature = "preserve_order")]
pub type MapIntoIter<K, V> = linked_hash_map::IntoIter<K, V>;

fn map_with_capacity<K: std::hash::Hash + Eq, V>(size: Option<usize>) -> Map<K, V> {
    #[cfg(not(feature = "preserve_order"))]
    {
        let _ = size;
        BTreeMap::new()
    }

    #[cfg(feature = "preserve_order")]
    {
        LinkedHashMap::with_capacity(size.unwrap_or(0))
    }
}

/// Represents a Hjson/JSON value
#[derive(Clone, PartialEq)]
pub enum Value {
    /// Represents a JSON null value
    Null,

    /// Represents a JSON Boolean
    Bool(bool),

    /// Represents a JSON signed integer
    I64(i64),

    /// Represents a JSON unsigned integer
    U64(u64),

    /// Represents a JSON floating point number
    F64(f64),

    /// Represents a JSON string
    String(String),

    /// Represents a JSON array
    Array(Vec<Value>),

    /// Represents a JSON object
    Object(Map<String, Value>),
}

impl Value {
    /// If the `Value` is an Object, returns the value associated with the provided key.
    /// Otherwise, returns None.
    pub fn find<'a>(&'a self, key: &str) -> Option<&'a Value> {
        match *self {
            Value::Object(ref map) => map.get(key),
            _ => None,
        }
    }

    /// Attempts to get a nested Value Object for each key in `keys`.
    /// If any key is found not to exist, find_path will return None.
    /// Otherwise, it will return the `Value` associated with the final key.
    pub fn find_path<'a>(&'a self, keys: &[&str]) -> Option<&'a Value> {
        let mut target = self;
        for key in keys {
            match target.find(key) {
                Some(t) => {
                    target = t;
                }
                None => return None,
            }
        }
        Some(target)
    }

    /// Looks up a value by a JSON Pointer.
    ///
    /// JSON Pointer defines a string syntax for identifying a specific value
    /// within a JavaScript Object Notation (JSON) document.
    ///
    /// A Pointer is a Unicode string with the reference tokens separated by `/`.
    /// Inside tokens `/` is replaced by `~1` and `~` is replaced by `~0`. The
    /// addressed value is returned and if there is no such value `None` is
    /// returned.
    ///
    /// For more information read [RFC6901](https://tools.ietf.org/html/rfc6901).
    pub fn pointer<'a>(&'a self, pointer: &str) -> Option<&'a Value> {
        fn parse_index(s: &str) -> Option<usize> {
            if s.starts_with('+') || (s.starts_with('0') && s.len() != 1) {
                return None;
            }
            s.parse().ok()
        }
        if pointer.is_empty() {
            return Some(self);
        }
        if !pointer.starts_with('/') {
            return None;
        }
        let mut target = self;
        for escaped_token in pointer.split('/').skip(1) {
            let token = escaped_token.replace("~1", "/").replace("~0", "~");
            let target_opt = match *target {
                Value::Object(ref map) => map.get(&token[..]),
                Value::Array(ref list) => parse_index(&token[..]).and_then(|x| list.get(x)),
                _ => return None,
            };
            if let Some(t) = target_opt {
                target = t;
            } else {
                return None;
            }
        }
        Some(target)
    }

    /// If the `Value` is an Object, performs a depth-first search until
    /// a value associated with the provided key is found. If no value is found
    /// or the `Value` is not an Object, returns None.
    pub fn search<'a>(&'a self, key: &str) -> Option<&'a Value> {
        match self {
            Value::Object(map) => map
                .get(key)
                .or_else(|| map.values().find_map(|v| v.search(key))),
            _ => None,
        }
    }

    /// Returns true if the `Value` is an Object. Returns false otherwise.
    pub fn is_object(&self) -> bool {
        self.as_object().is_some()
    }

    /// If the `Value` is an Object, returns the associated Map.
    /// Returns None otherwise.
    pub fn as_object(&self) -> Option<&Map<String, Value>> {
        match *self {
            Value::Object(ref map) => Some(map),
            _ => None,
        }
    }

    /// If the `Value` is an Object, returns the associated mutable Map.
    /// Returns None otherwise.
    pub fn as_object_mut(&mut self) -> Option<&mut Map<String, Value>> {
        match *self {
            Value::Object(ref mut map) => Some(map),
            _ => None,
        }
    }

    /// Returns true if the `Value` is an Array. Returns false otherwise.
    pub fn is_array(&self) -> bool {
        self.as_array().is_some()
    }

    /// If the `Value` is an Array, returns the associated vector.
    /// Returns None otherwise.
    pub fn as_array(&self) -> Option<&Vec<Value>> {
        match self {
            Value::Array(array) => Some(array),
            _ => None,
        }
    }

    /// If the `Value` is an Array, returns the associated mutable vector.
    /// Returns None otherwise.
    pub fn as_array_mut(&mut self) -> Option<&mut Vec<Value>> {
        match self {
            Value::Array(list) => Some(list),
            _ => None,
        }
    }

    /// Returns true if the `Value` is a String. Returns false otherwise.
    pub fn is_string(&self) -> bool {
        self.as_str().is_some()
    }

    /// If the `Value` is a String, returns the associated str.
    /// Returns None otherwise.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    /// Returns true if the `Value` is a Number. Returns false otherwise.
    pub fn is_number(&self) -> bool {
        matches!(self, Value::I64(_) | Value::U64(_) | Value::F64(_))
    }

    /// Returns true if the `Value` is a i64. Returns false otherwise.
    pub fn is_i64(&self) -> bool {
        matches!(self, Value::I64(_))
    }

    /// Returns true if the `Value` is a u64. Returns false otherwise.
    pub fn is_u64(&self) -> bool {
        matches!(self, Value::U64(_))
    }

    /// Returns true if the `Value` is a f64. Returns false otherwise.
    pub fn is_f64(&self) -> bool {
        matches!(self, Value::F64(_))
    }

    /// If the `Value` is a number, return or cast it to a i64.
    /// Returns None otherwise.
    pub fn as_i64(&self) -> Option<i64> {
        match *self {
            Value::I64(n) => Some(n),
            Value::U64(n) => NumCast::from(n),
            _ => None,
        }
    }

    /// If the `Value` is a number, return or cast it to a u64.
    /// Returns None otherwise.
    pub fn as_u64(&self) -> Option<u64> {
        match *self {
            Value::I64(n) => NumCast::from(n),
            Value::U64(n) => Some(n),
            _ => None,
        }
    }

    /// If the `Value` is a number, return or cast it to a f64.
    /// Returns None otherwise.
    pub fn as_f64(&self) -> Option<f64> {
        match *self {
            Value::I64(n) => NumCast::from(n),
            Value::U64(n) => NumCast::from(n),
            Value::F64(n) => Some(n),
            _ => None,
        }
    }

    /// Returns true if the `Value` is a Boolean. Returns false otherwise.
    pub fn is_boolean(&self) -> bool {
        self.as_bool().is_some()
    }

    /// If the `Value` is a Boolean, returns the associated bool.
    /// Returns None otherwise.
    pub fn as_bool(&self) -> Option<bool> {
        match *self {
            Value::Bool(b) => Some(b),
            _ => None,
        }
    }

    /// Returns true if the `Value` is a Null. Returns false otherwise.
    pub fn is_null(&self) -> bool {
        self.as_null().is_some()
    }

    /// If the `Value` is a Null, returns ().
    /// Returns None otherwise.
    pub fn as_null(&self) -> Option<()> {
        match self {
            Value::Null => Some(()),
            _ => None,
        }
    }

    fn as_unexpected(&self) -> de::Unexpected<'_> {
        match *self {
            Value::Null => de::Unexpected::Unit,
            Value::Bool(v) => de::Unexpected::Bool(v),
            Value::I64(v) => de::Unexpected::Signed(v),
            Value::U64(v) => de::Unexpected::Unsigned(v),
            Value::F64(v) => de::Unexpected::Float(v),
            Value::String(ref v) => de::Unexpected::Str(v),
            Value::Array(_) => de::Unexpected::Seq,
            Value::Object(_) => de::Unexpected::Map,
        }
    }
}

impl ser::Serialize for Value {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        match *self {
            Value::Null => serializer.serialize_unit(),
            Value::Bool(v) => serializer.serialize_bool(v),
            Value::I64(v) => serializer.serialize_i64(v),
            Value::U64(v) => serializer.serialize_u64(v),
            Value::F64(v) => serializer.serialize_f64(v),
            Value::String(ref v) => serializer.serialize_str(v),
            Value::Array(ref v) => v.serialize(serializer),
            Value::Object(ref v) => v.serialize(serializer),
        }
    }
}

impl<'de> de::Deserialize<'de> for Value {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Value, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct ValueVisitor;

        impl<'de> de::Visitor<'de> for ValueVisitor {
            type Value = Value;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a json value")
            }

            #[inline]
            fn visit_bool<E>(self, value: bool) -> Result<Value, E> {
                Ok(Value::Bool(value))
            }

            #[inline]
            fn visit_i64<E>(self, value: i64) -> Result<Value, E> {
                if value < 0 {
                    Ok(Value::I64(value))
                } else {
                    Ok(Value::U64(value as u64))
                }
            }

            #[inline]
            fn visit_u64<E>(self, value: u64) -> Result<Value, E> {
                Ok(Value::U64(value))
            }

            #[inline]
            fn visit_f64<E>(self, value: f64) -> Result<Value, E> {
                Ok(Value::F64(value))
            }

            #[inline]
            fn visit_str<E>(self, value: &str) -> Result<Value, E>
            where
                E: de::Error,
            {
                self.visit_string(String::from(value))
            }

            #[inline]
            fn visit_string<E>(self, value: String) -> Result<Value, E> {
                Ok(Value::String(value))
            }

            #[inline]
            fn visit_none<E>(self) -> Result<Value, E> {
                Ok(Value::Null)
            }

            #[inline]
            fn visit_some<D>(self, deserializer: D) -> Result<Value, D::Error>
            where
                D: de::Deserializer<'de>,
            {
                de::Deserialize::deserialize(deserializer)
            }

            #[inline]
            fn visit_unit<E>(self) -> Result<Value, E> {
                Ok(Value::Null)
            }

            #[inline]
            fn visit_seq<A>(self, mut seq: A) -> Result<Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let mut v = match seq.size_hint() {
                    Some(cap) => Vec::with_capacity(cap),
                    None => Vec::new(),
                };

                while let Some(el) = seq.next_element()? {
                    v.push(el)
                }

                Ok(Value::Array(v))
            }

            #[inline]
            fn visit_map<A>(self, mut map: A) -> Result<Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let mut values = map_with_capacity(map.size_hint());
                while let Some((k, v)) = map.next_entry()? {
                    values.insert(k, v);
                }
                Ok(Value::Object(values))
            }
        }

        deserializer.deserialize_any(ValueVisitor)
    }
}

struct WriterFormatter<'a, 'b: 'a> {
    inner: &'a mut fmt::Formatter<'b>,
}

impl<'a, 'b> io::Write for WriterFormatter<'a, 'b> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        fn io_error<E>(_: E) -> io::Error {
            // Value does not matter because fmt::Debug and fmt::Display impls
            // below just map it to fmt::Error
            io::Error::new(io::ErrorKind::Other, "fmt error")
        }
        let s = str::from_utf8(buf).map_err(io_error)?;
        self.inner.write_str(s).map_err(io_error)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl fmt::Debug for Value {
    /// Serializes a Hjson value into a string
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut wr = WriterFormatter { inner: f };
        super::ser::to_writer(&mut wr, self).map_err(|_| fmt::Error)
    }
}

impl fmt::Display for Value {
    /// Serializes a Hjson value into a string
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut wr = WriterFormatter { inner: f };
        super::ser::to_writer(&mut wr, self).map_err(|_| fmt::Error)
    }
}

impl str::FromStr for Value {
    type Err = Error;
    fn from_str(s: &str) -> Result<Value> {
        super::de::from_str(s)
    }
}

/// Create a `serde::Serializer` that serializes a `Serialize`e into a `Value`.
#[derive(Default)]
pub struct Serializer;

impl ser::Serializer for Serializer {
    type Ok = Value;
    type Error = Error;

    type SerializeSeq = SerializeVec;
    type SerializeTuple = SerializeVec;
    type SerializeTupleStruct = SerializeVec;
    type SerializeTupleVariant = SerializeTupleVariant;
    type SerializeMap = SerializeMap;
    type SerializeStruct = SerializeMap;
    type SerializeStructVariant = SerializeStructVariant;

    #[inline]
    fn serialize_bool(self, value: bool) -> Result<Value> {
        Ok(Value::Bool(value))
    }

    #[inline]
    fn serialize_i8(self, value: i8) -> Result<Value> {
        self.serialize_i64(value as i64)
    }

    #[inline]
    fn serialize_i16(self, value: i16) -> Result<Value> {
        self.serialize_i64(value as i64)
    }

    #[inline]
    fn serialize_i32(self, value: i32) -> Result<Value> {
        self.serialize_i64(value as i64)
    }

    fn serialize_i64(self, value: i64) -> Result<Value> {
        let v = if value < 0 {
            Value::I64(value)
        } else {
            Value::U64(value as u64)
        };
        Ok(v)
    }

    #[inline]
    fn serialize_u8(self, value: u8) -> Result<Value> {
        self.serialize_u64(value as u64)
    }

    #[inline]
    fn serialize_u16(self, value: u16) -> Result<Value> {
        self.serialize_u64(value as u64)
    }

    #[inline]
    fn serialize_u32(self, value: u32) -> Result<Value> {
        self.serialize_u64(value as u64)
    }

    #[inline]
    fn serialize_u64(self, value: u64) -> Result<Value> {
        Ok(Value::U64(value))
    }

    #[inline]
    fn serialize_f32(self, value: f32) -> Result<Value> {
        self.serialize_f64(value as f64)
    }

    #[inline]
    fn serialize_f64(self, value: f64) -> Result<Value> {
        Ok(Value::F64(value))
    }

    #[inline]
    fn serialize_char(self, value: char) -> Result<Value> {
        let mut s = String::new();
        s.push(value);
        self.serialize_str(&s)
    }

    #[inline]
    fn serialize_str(self, value: &str) -> Result<Value> {
        Ok(Value::String(String::from(value)))
    }

    fn serialize_bytes(self, value: &[u8]) -> Result<Value> {
        let mut state = self.serialize_seq(Some(value.len()))?;
        for byte in value {
            ser::SerializeSeq::serialize_element(&mut state, byte)?;
        }
        ser::SerializeSeq::end(state)
    }

    #[inline]
    fn serialize_unit(self) -> Result<Value> {
        Ok(Value::Null)
    }

    #[inline]
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Value> {
        self.serialize_unit()
    }

    #[inline]
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Value> {
        self.serialize_str(variant)
    }

    #[inline]
    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<Value>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Value>
    where
        T: ?Sized + ser::Serialize,
    {
        let mut values = Map::new();
        values.insert(String::from(variant), to_value(&value)?);
        Ok(Value::Object(values))
    }

    #[inline]
    fn serialize_none(self) -> Result<Value> {
        self.serialize_unit()
    }

    #[inline]
    fn serialize_some<V>(self, value: &V) -> Result<Value>
    where
        V: ?Sized + ser::Serialize,
    {
        value.serialize(self)
    }

    #[inline]
    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(SerializeVec {
            vec: Vec::with_capacity(len.unwrap_or(0)),
        })
    }

    #[inline]
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.serialize_seq(Some(len))
    }

    #[inline]
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.serialize_seq(Some(len))
    }

    #[inline]
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Ok(SerializeTupleVariant {
            name: variant,
            vec: Vec::with_capacity(len),
        })
    }

    #[inline]
    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(SerializeMap {
            map: map_with_capacity(len),
            next_key: None,
        })
    }

    #[inline]
    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        self.serialize_map(Some(len))
    }

    #[inline]
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Ok(SerializeStructVariant {
            name: variant,
            map: map_with_capacity(Some(len)),
        })
    }
}

#[doc(hidden)]
pub struct SerializeVec {
    vec: Vec<Value>,
}

#[doc(hidden)]
pub struct SerializeTupleVariant {
    name: &'static str,
    vec: Vec<Value>,
}

#[doc(hidden)]
pub struct SerializeMap {
    map: Map<String, Value>,
    next_key: Option<String>,
}

#[doc(hidden)]
pub struct SerializeStructVariant {
    name: &'static str,
    map: Map<String, Value>,
}

impl ser::SerializeSeq for SerializeVec {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        self.vec.push(to_value(&value)?);
        Ok(())
    }

    fn end(self) -> Result<Value> {
        Ok(Value::Array(self.vec))
    }
}

impl ser::SerializeTuple for SerializeVec {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Value> {
        ser::SerializeSeq::end(self)
    }
}

impl ser::SerializeTupleStruct for SerializeVec {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Value> {
        ser::SerializeSeq::end(self)
    }
}

impl ser::SerializeTupleVariant for SerializeTupleVariant {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        self.vec.push(to_value(&value)?);
        Ok(())
    }

    fn end(self) -> Result<Value> {
        let mut object = Map::new();

        object.insert(self.name.to_owned(), Value::Array(self.vec));

        Ok(Value::Object(object))
    }
}

impl ser::SerializeMap for SerializeMap {
    type Ok = Value;
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        match to_value(key)? {
            Value::String(s) => self.next_key = Some(s),
            _ => return Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0)),
        };
        Ok(())
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        let key = self.next_key.take();
        // Panic because this indicates a bug in the program rather than an
        // expected failure.
        let key = key.expect("serialize_value called before serialize_key");
        self.map.insert(key, to_value(value)?);
        Ok(())
    }

    fn end(self) -> Result<Value> {
        Ok(Value::Object(self.map))
    }
}

impl ser::SerializeStruct for SerializeMap {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        ser::SerializeMap::serialize_entry(self, key, value)
    }

    fn end(self) -> Result<Value> {
        ser::SerializeMap::end(self)
    }
}

impl ser::SerializeStructVariant for SerializeStructVariant {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        self.map.insert(key.to_owned(), to_value(&value)?);
        Ok(())
    }

    fn end(self) -> Result<Value> {
        let mut object = map_with_capacity(Some(1));

        object.insert(self.name.to_owned(), Value::Object(self.map));

        Ok(Value::Object(object))
    }
}

impl<'de> de::Deserializer<'de> for Value {
    type Error = Error;

    #[inline]
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match self {
            Value::Null => visitor.visit_unit(),
            Value::Bool(v) => visitor.visit_bool(v),
            Value::I64(v) => visitor.visit_i64(v),
            Value::U64(v) => visitor.visit_u64(v),
            Value::F64(v) => visitor.visit_f64(v),
            Value::String(v) => visitor.visit_string(v),
            Value::Array(v) => visitor.visit_seq(SeqDeserializer {
                iter: v.into_iter(),
            }),
            Value::Object(v) => visitor.visit_map(MapDeserializer {
                iter: v.into_iter(),
                value: None,
            }),
        }
    }

    #[inline]
    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match self {
            Value::Null => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }

    #[inline]
    fn deserialize_enum<V>(
        self,
        _name: &str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        let (variant, value) = match self {
            Value::Object(value) => {
                let mut iter = value.into_iter();
                let (variant, value) = match iter.next() {
                    Some(v) => v,
                    None => {
                        return Err(de::Error::invalid_type(
                            de::Unexpected::Map,
                            &"map with a single key",
                        ));
                    }
                };
                // enums are encoded in json as maps with a single key:value pair
                if iter.next().is_some() {
                    return Err(de::Error::invalid_type(
                        de::Unexpected::Map,
                        &"map with a single key",
                    ));
                }
                (variant, Some(value))
            }
            Value::String(variant) => (variant, None),
            val => {
                return Err(de::Error::invalid_type(
                    val.as_unexpected(),
                    &"string or map",
                ))
            }
        };

        visitor.visit_enum(EnumDeserializer { variant, value })
    }

    #[inline]
    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf unit unit_struct seq tuple
        tuple_struct map struct identifier ignored_any
    }
}

struct EnumDeserializer {
    variant: String,
    value: Option<Value>,
}

impl<'de> de::EnumAccess<'de> for EnumDeserializer {
    type Error = Error;

    type Variant = VariantDeserializer;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        let variant = de::IntoDeserializer::into_deserializer(self.variant);
        let visitor = VariantDeserializer { val: self.value };
        seed.deserialize(variant).map(|v| (v, visitor))
    }
}

struct VariantDeserializer {
    val: Option<Value>,
}

impl<'de> de::VariantAccess<'de> for VariantDeserializer {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        match self.val {
            Some(val) => de::Deserialize::deserialize(val),
            None => Ok(()),
        }
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: de::DeserializeSeed<'de>,
    {
        match self.val {
            Some(value) => seed.deserialize(value),
            None => Err(serde::de::Error::invalid_type(
                de::Unexpected::UnitVariant,
                &"newtype variant",
            )),
        }
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        let val = self.val.expect("val is missing");
        if let Value::Array(fields) = val {
            visitor.visit_seq(SeqDeserializer {
                iter: fields.into_iter(),
            })
        } else {
            Err(de::Error::invalid_type(val.as_unexpected(), &visitor))
        }
    }

    fn struct_variant<V>(self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match self.val {
            Some(Value::Object(fields)) => visitor.visit_map(MapDeserializer {
                iter: fields.into_iter(),
                value: None,
            }),
            Some(other) => Err(de::Error::invalid_type(
                other.as_unexpected(),
                &"struct variant",
            )),
            None => Err(de::Error::invalid_type(
                de::Unexpected::UnitVariant,
                &"struct variant",
            )),
        }
    }
}

struct SeqDeserializer {
    iter: vec::IntoIter<Value>,
}

impl<'de> de::SeqAccess<'de> for SeqDeserializer {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(value) => Ok(Some(seed.deserialize(value)?)),
            None => Ok(None),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        match self.iter.size_hint() {
            (lower, Some(upper)) if lower == upper => Some(upper),
            _ => None,
        }
    }
}

struct MapDeserializer {
    iter: MapIntoIter<String, Value>,
    value: Option<Value>,
}

impl<'de> de::MapAccess<'de> for MapDeserializer {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: de::DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some((key, value)) => {
                self.value = Some(value);
                Ok(Some(seed.deserialize(Value::String(key))?))
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        let value = self.value.take().expect("value is missing");
        seed.deserialize(value)
    }

    fn size_hint(&self) -> Option<usize> {
        match self.iter.size_hint() {
            (lower, Some(upper)) if lower == upper => Some(upper),
            _ => None,
        }
    }
}

pub fn to_value<T: ?Sized>(value: &T) -> Result<Value>
where
    T: ser::Serialize,
{
    value.serialize(Serializer)
}

/// Shortcut function to decode a Hjson `Value` into a `T`
pub fn from_value<T>(value: Value) -> Result<T>
where
    T: de::DeserializeOwned,
{
    de::Deserialize::deserialize(value)
}

/// A trait for converting values to Hjson
pub trait ToJson {
    /// Converts the value of `self` to an instance of Hjson
    fn to_json(&self) -> Value;
}

impl<T: ?Sized> ToJson for T
where
    T: ser::Serialize,
{
    fn to_json(&self) -> Value {
        to_value(&self).expect("failed to serialize")
    }
}

#[cfg(test)]
mod test {
    use super::Value;
    use crate::de::from_str;

    #[test]
    fn number_deserialize() {
        let v: Value = from_str("{\"a\":1}").unwrap();
        let vo = v.as_object().unwrap();
        assert_eq!(vo["a"].as_u64().unwrap(), 1);

        let v: Value = from_str("{\"a\":-1}").unwrap();
        let vo = v.as_object().unwrap();
        assert_eq!(vo["a"].as_i64().unwrap(), -1);

        let v: Value = from_str("{\"a\":1.1}").unwrap();
        let vo = v.as_object().unwrap();
        assert!(vo["a"].as_f64().unwrap() - 1.1 < std::f64::EPSILON);

        let v: Value = from_str("{\"a\":-1.1}").unwrap();
        let vo = v.as_object().unwrap();
        assert!(vo["a"].as_f64().unwrap() + 1.1 > -(std::f64::EPSILON));

        let v: Value = from_str("{\"a\":1e6}").unwrap();
        let vo = v.as_object().unwrap();
        assert!(vo["a"].as_f64().unwrap() - 1e6 < std::f64::EPSILON);

        let v: Value = from_str("{\"a\":-1e6}").unwrap();
        let vo = v.as_object().unwrap();
        assert!(vo["a"].as_f64().unwrap() + 1e6 > -(std::f64::EPSILON));
    }
}
