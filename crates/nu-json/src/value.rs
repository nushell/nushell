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

#[cfg(not(feature = "preserve_order"))]
type MapVisitor<K, T> = de::impls::BTreeMapVisitor<K, T>;
#[cfg(feature = "preserve_order")]
type MapVisitor<K, T> = linked_hash_map::serde::LinkedHashMapVisitor<K, T>;

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
        match *self {
            Value::Object(ref map) => match map.get(key) {
                Some(json_value) => Some(json_value),
                None => {
                    for (_, v) in map.iter() {
                        match v.search(key) {
                            x if x.is_some() => return x,
                            _ => (),
                        }
                    }
                    None
                }
            },
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
        match *self {
            Value::Array(ref array) => Some(&*array),
            _ => None,
        }
    }

    /// If the `Value` is an Array, returns the associated mutable vector.
    /// Returns None otherwise.
    pub fn as_array_mut(&mut self) -> Option<&mut Vec<Value>> {
        match *self {
            Value::Array(ref mut list) => Some(list),
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
        match *self {
            Value::String(ref s) => Some(s),
            _ => None,
        }
    }

    /// Returns true if the `Value` is a Number. Returns false otherwise.
    pub fn is_number(&self) -> bool {
        matches!(*self, Value::I64(_) | Value::U64(_) | Value::F64(_))
    }

    /// Returns true if the `Value` is a i64. Returns false otherwise.
    pub fn is_i64(&self) -> bool {
        matches!(*self, Value::I64(_))
    }

    /// Returns true if the `Value` is a u64. Returns false otherwise.
    pub fn is_u64(&self) -> bool {
        matches!(*self, Value::U64(_))
    }

    /// Returns true if the `Value` is a f64. Returns false otherwise.
    pub fn is_f64(&self) -> bool {
        matches!(*self, Value::F64(_))
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
        match *self {
            Value::Null => Some(()),
            _ => None,
        }
    }
}

impl ser::Serialize for Value {
    #[inline]
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
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

impl de::Deserialize for Value {
    #[inline]
    fn deserialize<D>(deserializer: &mut D) -> Result<Value, D::Error>
    where
        D: de::Deserializer,
    {
        struct ValueVisitor;

        impl de::Visitor for ValueVisitor {
            type Value = Value;

            #[inline]
            fn visit_bool<E>(&mut self, value: bool) -> Result<Value, E> {
                Ok(Value::Bool(value))
            }

            #[inline]
            fn visit_i64<E>(&mut self, value: i64) -> Result<Value, E> {
                if value < 0 {
                    Ok(Value::I64(value))
                } else {
                    Ok(Value::U64(value as u64))
                }
            }

            #[inline]
            fn visit_u64<E>(&mut self, value: u64) -> Result<Value, E> {
                Ok(Value::U64(value))
            }

            #[inline]
            fn visit_f64<E>(&mut self, value: f64) -> Result<Value, E> {
                Ok(Value::F64(value))
            }

            #[inline]
            fn visit_str<E>(&mut self, value: &str) -> Result<Value, E>
            where
                E: de::Error,
            {
                self.visit_string(String::from(value))
            }

            #[inline]
            fn visit_string<E>(&mut self, value: String) -> Result<Value, E> {
                Ok(Value::String(value))
            }

            #[inline]
            fn visit_none<E>(&mut self) -> Result<Value, E> {
                Ok(Value::Null)
            }

            #[inline]
            fn visit_some<D>(&mut self, deserializer: &mut D) -> Result<Value, D::Error>
            where
                D: de::Deserializer,
            {
                de::Deserialize::deserialize(deserializer)
            }

            #[inline]
            fn visit_unit<E>(&mut self) -> Result<Value, E> {
                Ok(Value::Null)
            }

            #[inline]
            fn visit_seq<V>(&mut self, visitor: V) -> Result<Value, V::Error>
            where
                V: de::SeqVisitor,
            {
                let values = de::impls::VecVisitor::new().visit_seq(visitor)?;
                Ok(Value::Array(values))
            }

            #[inline]
            fn visit_map<V>(&mut self, visitor: V) -> Result<Value, V::Error>
            where
                V: de::MapVisitor,
            {
                let values = MapVisitor::new().visit_map(visitor)?;
                Ok(Value::Object(values))
            }
        }

        deserializer.deserialize(ValueVisitor)
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
    fn from_str(s: &str) -> Result<Value, Error> {
        super::de::from_str(s)
    }
}

/// Create a `serde::Serializer` that serializes a `Serialize`e into a `Value`.
pub struct Serializer {
    value: Value,
}

impl Serializer {
    /// Construct a new `Serializer`.
    pub fn new() -> Serializer {
        Serializer { value: Value::Null }
    }

    /// Unwrap the `Serializer` and return the `Value`.
    pub fn unwrap(self) -> Value {
        self.value
    }
}

impl Default for Serializer {
    fn default() -> Self {
        Serializer::new()
    }
}

#[doc(hidden)]
pub struct TupleVariantState {
    name: String,
    vec: Vec<Value>,
}

#[doc(hidden)]
pub struct StructVariantState {
    name: String,
    map: Map<String, Value>,
}

#[doc(hidden)]
pub struct MapState {
    map: Map<String, Value>,
    next_key: Option<String>,
}

impl ser::Serializer for Serializer {
    type Error = Error;

    type SeqState = Vec<Value>;
    type TupleState = Vec<Value>;
    type TupleStructState = Vec<Value>;
    type TupleVariantState = TupleVariantState;
    type MapState = MapState;
    type StructState = MapState;
    type StructVariantState = StructVariantState;

    #[inline]
    fn serialize_bool(&mut self, value: bool) -> Result<(), Error> {
        self.value = Value::Bool(value);
        Ok(())
    }

    #[inline]
    fn serialize_isize(&mut self, value: isize) -> Result<(), Error> {
        self.serialize_i64(value as i64)
    }

    #[inline]
    fn serialize_i8(&mut self, value: i8) -> Result<(), Error> {
        self.serialize_i64(value as i64)
    }

    #[inline]
    fn serialize_i16(&mut self, value: i16) -> Result<(), Error> {
        self.serialize_i64(value as i64)
    }

    #[inline]
    fn serialize_i32(&mut self, value: i32) -> Result<(), Error> {
        self.serialize_i64(value as i64)
    }

    fn serialize_i64(&mut self, value: i64) -> Result<(), Error> {
        if value < 0 {
            self.value = Value::I64(value);
        } else {
            self.value = Value::U64(value as u64);
        }
        Ok(())
    }

    #[inline]
    fn serialize_usize(&mut self, value: usize) -> Result<(), Error> {
        self.serialize_u64(value as u64)
    }

    #[inline]
    fn serialize_u8(&mut self, value: u8) -> Result<(), Error> {
        self.serialize_u64(value as u64)
    }

    #[inline]
    fn serialize_u16(&mut self, value: u16) -> Result<(), Error> {
        self.serialize_u64(value as u64)
    }

    #[inline]
    fn serialize_u32(&mut self, value: u32) -> Result<(), Error> {
        self.serialize_u64(value as u64)
    }

    #[inline]
    fn serialize_u64(&mut self, value: u64) -> Result<(), Error> {
        self.value = Value::U64(value);
        Ok(())
    }

    #[inline]
    fn serialize_f32(&mut self, value: f32) -> Result<(), Error> {
        self.serialize_f64(value as f64)
    }

    #[inline]
    fn serialize_f64(&mut self, value: f64) -> Result<(), Error> {
        self.value = Value::F64(value);
        Ok(())
    }

    #[inline]
    fn serialize_char(&mut self, value: char) -> Result<(), Error> {
        let mut s = String::new();
        s.push(value);
        self.serialize_str(&s)
    }

    #[inline]
    fn serialize_str(&mut self, value: &str) -> Result<(), Error> {
        self.value = Value::String(String::from(value));
        Ok(())
    }

    fn serialize_bytes(&mut self, value: &[u8]) -> Result<(), Error> {
        let mut state = self.serialize_seq(Some(value.len()))?;
        for byte in value {
            self.serialize_seq_elt(&mut state, byte)?;
        }
        self.serialize_seq_end(state)
    }

    #[inline]
    fn serialize_unit(&mut self) -> Result<(), Error> {
        Ok(())
    }

    #[inline]
    fn serialize_unit_struct(&mut self, _name: &'static str) -> Result<(), Error> {
        self.serialize_unit()
    }

    #[inline]
    fn serialize_unit_variant(
        &mut self,
        _name: &'static str,
        _variant_index: usize,
        variant: &'static str,
    ) -> Result<(), Error> {
        self.serialize_str(variant)
    }

    #[inline]
    fn serialize_newtype_struct<T>(&mut self, _name: &'static str, value: T) -> Result<(), Error>
    where
        T: ser::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        &mut self,
        _name: &'static str,
        _variant_index: usize,
        variant: &'static str,
        value: T,
    ) -> Result<(), Error>
    where
        T: ser::Serialize,
    {
        let mut values = Map::new();
        values.insert(String::from(variant), to_value(&value));
        self.value = Value::Object(values);
        Ok(())
    }

    #[inline]
    fn serialize_none(&mut self) -> Result<(), Error> {
        self.serialize_unit()
    }

    #[inline]
    fn serialize_some<V>(&mut self, value: V) -> Result<(), Error>
    where
        V: ser::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_seq(&mut self, len: Option<usize>) -> Result<Vec<Value>, Error> {
        Ok(Vec::with_capacity(len.unwrap_or(0)))
    }

    fn serialize_seq_elt<T: ser::Serialize>(
        &mut self,
        state: &mut Vec<Value>,
        value: T,
    ) -> Result<(), Error>
    where
        T: ser::Serialize,
    {
        state.push(to_value(&value));
        Ok(())
    }

    fn serialize_seq_end(&mut self, state: Vec<Value>) -> Result<(), Error> {
        self.value = Value::Array(state);
        Ok(())
    }

    fn serialize_seq_fixed_size(&mut self, size: usize) -> Result<Vec<Value>, Error> {
        self.serialize_seq(Some(size))
    }

    fn serialize_tuple(&mut self, len: usize) -> Result<Vec<Value>, Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_elt<T: ser::Serialize>(
        &mut self,
        state: &mut Vec<Value>,
        value: T,
    ) -> Result<(), Error> {
        self.serialize_seq_elt(state, value)
    }

    fn serialize_tuple_end(&mut self, state: Vec<Value>) -> Result<(), Error> {
        self.serialize_seq_end(state)
    }

    fn serialize_tuple_struct(
        &mut self,
        _name: &'static str,
        len: usize,
    ) -> Result<Vec<Value>, Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct_elt<T: ser::Serialize>(
        &mut self,
        state: &mut Vec<Value>,
        value: T,
    ) -> Result<(), Error> {
        self.serialize_seq_elt(state, value)
    }

    fn serialize_tuple_struct_end(&mut self, state: Vec<Value>) -> Result<(), Error> {
        self.serialize_seq_end(state)
    }

    fn serialize_tuple_variant(
        &mut self,
        _name: &'static str,
        _variant_index: usize,
        variant: &'static str,
        len: usize,
    ) -> Result<TupleVariantState, Error> {
        Ok(TupleVariantState {
            name: String::from(variant),
            vec: Vec::with_capacity(len),
        })
    }

    fn serialize_tuple_variant_elt<T: ser::Serialize>(
        &mut self,
        state: &mut TupleVariantState,
        value: T,
    ) -> Result<(), Error> {
        state.vec.push(to_value(&value));
        Ok(())
    }

    fn serialize_tuple_variant_end(&mut self, state: TupleVariantState) -> Result<(), Error> {
        let mut object = Map::new();

        object.insert(state.name, Value::Array(state.vec));

        self.value = Value::Object(object);
        Ok(())
    }

    fn serialize_map(&mut self, _len: Option<usize>) -> Result<MapState, Error> {
        Ok(MapState {
            map: Map::new(),
            next_key: None,
        })
    }

    fn serialize_map_key<T: ser::Serialize>(
        &mut self,
        state: &mut MapState,
        key: T,
    ) -> Result<(), Error> {
        match to_value(&key) {
            Value::String(s) => state.next_key = Some(s),
            _ => return Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0)),
        };
        Ok(())
    }

    fn serialize_map_value<T: ser::Serialize>(
        &mut self,
        state: &mut MapState,
        value: T,
    ) -> Result<(), Error> {
        match state.next_key.take() {
            Some(key) => state.map.insert(key, to_value(&value)),
            None => {
                return Err(Error::Syntax(
                    ErrorCode::Custom(
                        "serialize_map_value without \
                                                            matching serialize_map_key"
                            .to_owned(),
                    ),
                    0,
                    0,
                ));
            }
        };
        Ok(())
    }

    fn serialize_map_end(&mut self, state: MapState) -> Result<(), Error> {
        self.value = Value::Object(state.map);
        Ok(())
    }

    fn serialize_struct(&mut self, _name: &'static str, len: usize) -> Result<MapState, Error> {
        self.serialize_map(Some(len))
    }

    fn serialize_struct_elt<V: ser::Serialize>(
        &mut self,
        state: &mut MapState,
        key: &'static str,
        value: V,
    ) -> Result<(), Error> {
        self.serialize_map_key(state, key)?;
        self.serialize_map_value(state, value)
    }

    fn serialize_struct_end(&mut self, state: MapState) -> Result<(), Error> {
        self.serialize_map_end(state)
    }

    fn serialize_struct_variant(
        &mut self,
        _name: &'static str,
        _variant_index: usize,
        variant: &'static str,
        _len: usize,
    ) -> Result<StructVariantState, Error> {
        Ok(StructVariantState {
            name: String::from(variant),
            map: Map::new(),
        })
    }

    fn serialize_struct_variant_elt<V: ser::Serialize>(
        &mut self,
        state: &mut StructVariantState,
        key: &'static str,
        value: V,
    ) -> Result<(), Error> {
        state.map.insert(String::from(key), to_value(&value));
        Ok(())
    }

    fn serialize_struct_variant_end(&mut self, state: StructVariantState) -> Result<(), Error> {
        let mut object = Map::new();

        object.insert(state.name, Value::Object(state.map));

        self.value = Value::Object(object);
        Ok(())
    }
}

/// Creates a `serde::Deserializer` from a `Value` object.
pub struct Deserializer {
    value: Option<Value>,
}

impl Deserializer {
    /// Creates a new deserializer instance for deserializing the specified Hjson value.
    pub fn new(value: Value) -> Deserializer {
        Deserializer { value: Some(value) }
    }
}

impl de::Deserializer for Deserializer {
    type Error = Error;

    #[inline]
    fn deserialize<V>(&mut self, mut visitor: V) -> Result<V::Value, Error>
    where
        V: de::Visitor,
    {
        let value = match self.value.take() {
            Some(value) => value,
            None => {
                return Err(de::Error::end_of_stream());
            }
        };

        match value {
            Value::Null => visitor.visit_unit(),
            Value::Bool(v) => visitor.visit_bool(v),
            Value::I64(v) => visitor.visit_i64(v),
            Value::U64(v) => visitor.visit_u64(v),
            Value::F64(v) => visitor.visit_f64(v),
            Value::String(v) => visitor.visit_string(v),
            Value::Array(v) => {
                let len = v.len();
                visitor.visit_seq(SeqDeserializer {
                    de: self,
                    iter: v.into_iter(),
                    len,
                })
            }
            Value::Object(v) => {
                let len = v.len();
                visitor.visit_map(MapDeserializer {
                    de: self,
                    iter: v.into_iter(),
                    value: None,
                    len,
                })
            }
        }
    }

    #[inline]
    fn deserialize_option<V>(&mut self, mut visitor: V) -> Result<V::Value, Error>
    where
        V: de::Visitor,
    {
        match self.value {
            Some(Value::Null) => visitor.visit_none(),
            Some(_) => visitor.visit_some(self),
            None => Err(de::Error::end_of_stream()),
        }
    }

    #[inline]
    fn deserialize_enum<V>(
        &mut self,
        _name: &str,
        _variants: &'static [&'static str],
        mut visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: de::EnumVisitor,
    {
        let (variant, value) = match self.value.take() {
            Some(Value::Object(value)) => {
                let mut iter = value.into_iter();
                let (variant, value) = match iter.next() {
                    Some(v) => v,
                    None => {
                        return Err(de::Error::invalid_type(de::Type::VariantName));
                    }
                };
                // enums are encoded in json as maps with a single key:value pair
                if iter.next().is_some() {
                    return Err(de::Error::invalid_type(de::Type::Map));
                }
                (variant, Some(value))
            }
            Some(Value::String(variant)) => (variant, None),
            Some(_) => {
                return Err(de::Error::invalid_type(de::Type::Enum));
            }
            None => {
                return Err(de::Error::end_of_stream());
            }
        };

        visitor.visit(VariantDeserializer {
            de: self,
            val: value,
            variant: Some(Value::String(variant)),
        })
    }

    #[inline]
    fn deserialize_newtype_struct<V>(
        &mut self,
        _name: &'static str,
        mut visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor,
    {
        visitor.visit_newtype_struct(self)
    }

    forward_to_deserialize! {
        deserialize_bool();
        deserialize_usize();
        deserialize_u8();
        deserialize_u16();
        deserialize_u32();
        deserialize_u64();
        deserialize_isize();
        deserialize_i8();
        deserialize_i16();
        deserialize_i32();
        deserialize_i64();
        deserialize_f32();
        deserialize_f64();
        deserialize_char();
        deserialize_str();
        deserialize_string();
        deserialize_unit();
        deserialize_seq();
        deserialize_seq_fixed_size(len: usize);
        deserialize_bytes();
        deserialize_map();
        deserialize_unit_struct(name: &'static str);
        deserialize_tuple_struct(name: &'static str, len: usize);
        deserialize_struct(name: &'static str, fields: &'static [&'static str]);
        deserialize_struct_field();
        deserialize_tuple(len: usize);
        deserialize_ignored_any();
    }
}

struct VariantDeserializer<'a> {
    de: &'a mut Deserializer,
    val: Option<Value>,
    variant: Option<Value>,
}

impl<'a> de::VariantVisitor for VariantDeserializer<'a> {
    type Error = Error;

    fn visit_variant<V>(&mut self) -> Result<V, Error>
    where
        V: de::Deserialize,
    {
        let variant = self.variant.take().expect("variant is missing");
        de::Deserialize::deserialize(&mut Deserializer::new(variant))
    }

    fn visit_unit(&mut self) -> Result<(), Error> {
        match self.val.take() {
            Some(val) => de::Deserialize::deserialize(&mut Deserializer::new(val)),
            None => Ok(()),
        }
    }

    fn visit_newtype<T>(&mut self) -> Result<T, Error>
    where
        T: de::Deserialize,
    {
        let val = self.val.take().expect("val is missing");
        de::Deserialize::deserialize(&mut Deserializer::new(val))
    }

    fn visit_tuple<V>(&mut self, _len: usize, visitor: V) -> Result<V::Value, Error>
    where
        V: de::Visitor,
    {
        let val = self.val.take().expect("val is missing");
        if let Value::Array(fields) = val {
            de::Deserializer::deserialize(
                &mut SeqDeserializer {
                    de: self.de,
                    len: fields.len(),
                    iter: fields.into_iter(),
                },
                visitor,
            )
        } else {
            Err(de::Error::invalid_type(de::Type::Tuple))
        }
    }

    fn visit_struct<V>(
        &mut self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: de::Visitor,
    {
        let val = self.val.take().expect("val is missing");
        if let Value::Object(fields) = val {
            de::Deserializer::deserialize(
                &mut MapDeserializer {
                    de: self.de,
                    len: fields.len(),
                    iter: fields.into_iter(),
                    value: None,
                },
                visitor,
            )
        } else {
            Err(de::Error::invalid_type(de::Type::Struct))
        }
    }
}

struct SeqDeserializer<'a> {
    de: &'a mut Deserializer,
    iter: vec::IntoIter<Value>,
    len: usize,
}

impl<'a> de::Deserializer for SeqDeserializer<'a> {
    type Error = Error;

    #[inline]
    fn deserialize<V>(&mut self, mut visitor: V) -> Result<V::Value, Error>
    where
        V: de::Visitor,
    {
        if self.len == 0 {
            visitor.visit_unit()
        } else {
            visitor.visit_seq(self)
        }
    }

    forward_to_deserialize! {
        deserialize_bool();
        deserialize_usize();
        deserialize_u8();
        deserialize_u16();
        deserialize_u32();
        deserialize_u64();
        deserialize_isize();
        deserialize_i8();
        deserialize_i16();
        deserialize_i32();
        deserialize_i64();
        deserialize_f32();
        deserialize_f64();
        deserialize_char();
        deserialize_str();
        deserialize_string();
        deserialize_unit();
        deserialize_option();
        deserialize_seq();
        deserialize_seq_fixed_size(len: usize);
        deserialize_bytes();
        deserialize_map();
        deserialize_unit_struct(name: &'static str);
        deserialize_newtype_struct(name: &'static str);
        deserialize_tuple_struct(name: &'static str, len: usize);
        deserialize_struct(name: &'static str, fields: &'static [&'static str]);
        deserialize_struct_field();
        deserialize_tuple(len: usize);
        deserialize_enum(name: &'static str, variants: &'static [&'static str]);
        deserialize_ignored_any();
    }
}

impl<'a> de::SeqVisitor for SeqDeserializer<'a> {
    type Error = Error;

    fn visit<T>(&mut self) -> Result<Option<T>, Error>
    where
        T: de::Deserialize,
    {
        match self.iter.next() {
            Some(value) => {
                self.len -= 1;
                self.de.value = Some(value);
                Ok(Some(de::Deserialize::deserialize(self.de)?))
            }
            None => Ok(None),
        }
    }

    fn end(&mut self) -> Result<(), Error> {
        if self.len == 0 {
            Ok(())
        } else {
            Err(de::Error::invalid_length(self.len))
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

struct MapDeserializer<'a> {
    de: &'a mut Deserializer,
    iter: MapIntoIter<String, Value>,
    value: Option<Value>,
    len: usize,
}

impl<'a> de::MapVisitor for MapDeserializer<'a> {
    type Error = Error;

    fn visit_key<T>(&mut self) -> Result<Option<T>, Error>
    where
        T: de::Deserialize,
    {
        match self.iter.next() {
            Some((key, value)) => {
                self.len -= 1;
                self.value = Some(value);
                self.de.value = Some(Value::String(key));
                Ok(Some(de::Deserialize::deserialize(self.de)?))
            }
            None => Ok(None),
        }
    }

    fn visit_value<T>(&mut self) -> Result<T, Error>
    where
        T: de::Deserialize,
    {
        let value = self.value.take().expect("value is missing");
        self.de.value = Some(value);
        de::Deserialize::deserialize(self.de)
    }

    fn end(&mut self) -> Result<(), Error> {
        if self.len == 0 {
            Ok(())
        } else {
            Err(de::Error::invalid_length(self.len))
        }
    }

    fn missing_field<V>(&mut self, field: &'static str) -> Result<V, Error>
    where
        V: de::Deserialize,
    {
        struct MissingFieldDeserializer(&'static str);

        impl de::Deserializer for MissingFieldDeserializer {
            type Error = de::value::Error;

            fn deserialize<V>(&mut self, _visitor: V) -> Result<V::Value, Self::Error>
            where
                V: de::Visitor,
            {
                let &mut MissingFieldDeserializer(field) = self;
                Err(de::value::Error::MissingField(field))
            }

            fn deserialize_option<V>(&mut self, mut visitor: V) -> Result<V::Value, Self::Error>
            where
                V: de::Visitor,
            {
                visitor.visit_none()
            }

            forward_to_deserialize! {
                deserialize_bool();
                deserialize_usize();
                deserialize_u8();
                deserialize_u16();
                deserialize_u32();
                deserialize_u64();
                deserialize_isize();
                deserialize_i8();
                deserialize_i16();
                deserialize_i32();
                deserialize_i64();
                deserialize_f32();
                deserialize_f64();
                deserialize_char();
                deserialize_str();
                deserialize_string();
                deserialize_unit();
                deserialize_seq();
                deserialize_seq_fixed_size(len: usize);
                deserialize_bytes();
                deserialize_map();
                deserialize_unit_struct(name: &'static str);
                deserialize_newtype_struct(name: &'static str);
                deserialize_tuple_struct(name: &'static str, len: usize);
                deserialize_struct(name: &'static str, fields: &'static [&'static str]);
                deserialize_struct_field();
                deserialize_tuple(len: usize);
                deserialize_enum(name: &'static str, variants: &'static [&'static str]);
                deserialize_ignored_any();
            }
        }

        let mut de = MissingFieldDeserializer(field);
        Ok(de::Deserialize::deserialize(&mut de)?)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<'a> de::Deserializer for MapDeserializer<'a> {
    type Error = Error;

    #[inline]
    fn deserialize<V>(&mut self, mut visitor: V) -> Result<V::Value, Error>
    where
        V: de::Visitor,
    {
        visitor.visit_map(self)
    }

    forward_to_deserialize! {
        deserialize_bool();
        deserialize_usize();
        deserialize_u8();
        deserialize_u16();
        deserialize_u32();
        deserialize_u64();
        deserialize_isize();
        deserialize_i8();
        deserialize_i16();
        deserialize_i32();
        deserialize_i64();
        deserialize_f32();
        deserialize_f64();
        deserialize_char();
        deserialize_str();
        deserialize_string();
        deserialize_unit();
        deserialize_option();
        deserialize_seq();
        deserialize_seq_fixed_size(len: usize);
        deserialize_bytes();
        deserialize_map();
        deserialize_unit_struct(name: &'static str);
        deserialize_newtype_struct(name: &'static str);
        deserialize_tuple_struct(name: &'static str, len: usize);
        deserialize_struct(name: &'static str, fields: &'static [&'static str]);
        deserialize_struct_field();
        deserialize_tuple(len: usize);
        deserialize_enum(name: &'static str, variants: &'static [&'static str]);
        deserialize_ignored_any();
    }
}

pub fn to_value<T: ?Sized>(value: &T) -> Value
where
    T: ser::Serialize,
{
    let mut ser = Serializer::new();
    value.serialize(&mut ser).expect("failed to serialize");
    ser.unwrap()
}

/// Shortcut function to decode a Hjson `Value` into a `T`
pub fn from_value<T>(value: Value) -> Result<T, Error>
where
    T: de::Deserialize,
{
    let mut de = Deserializer::new(value);
    de::Deserialize::deserialize(&mut de)
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
        to_value(&self)
    }
}

#[cfg(test)]
mod test {
    use super::Value;
    use crate::de::from_str;

    #[test]
    fn number_deserialize() {
        let v: Value = from_str("{\"a\":1}").expect("Internal error: json parsing");
        let vo = v.as_object().expect("Internal error: json parsing");
        assert_eq!(vo["a"].as_u64().expect("Internal error: json parsing"), 1);

        let v: Value = from_str("{\"a\":-1}").expect("Internal error: json parsing");
        let vo = v.as_object().expect("Internal error: json parsing");
        assert_eq!(vo["a"].as_i64().expect("Internal error: json parsing"), -1);
    }
}
