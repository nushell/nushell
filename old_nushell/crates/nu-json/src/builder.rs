use serde::ser;

use crate::value::{self, Map, Value};

/// This structure provides a simple interface for constructing a JSON array.
pub struct ArrayBuilder {
    array: Vec<Value>,
}

impl Default for ArrayBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ArrayBuilder {
    /// Construct an `ObjectBuilder`.
    pub fn new() -> ArrayBuilder {
        ArrayBuilder { array: Vec::new() }
    }

    /// Return the constructed `Value`.
    pub fn unwrap(self) -> Value {
        Value::Array(self.array)
    }

    /// Insert a value into the array.
    pub fn push<T: ser::Serialize>(mut self, v: T) -> ArrayBuilder {
        self.array
            .push(value::to_value(&v).expect("failed to serialize"));
        self
    }

    /// Creates and passes an `ArrayBuilder` into a closure, then inserts the resulting array into
    /// this array.
    pub fn push_array<F>(mut self, f: F) -> ArrayBuilder
    where
        F: FnOnce(ArrayBuilder) -> ArrayBuilder,
    {
        let builder = ArrayBuilder::new();
        self.array.push(f(builder).unwrap());
        self
    }

    /// Creates and passes an `ArrayBuilder` into a closure, then inserts the resulting object into
    /// this array.
    pub fn push_object<F>(mut self, f: F) -> ArrayBuilder
    where
        F: FnOnce(ObjectBuilder) -> ObjectBuilder,
    {
        let builder = ObjectBuilder::new();
        self.array.push(f(builder).unwrap());
        self
    }
}

/// This structure provides a simple interface for constructing a JSON object.
pub struct ObjectBuilder {
    object: Map<String, Value>,
}

impl Default for ObjectBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ObjectBuilder {
    /// Construct an `ObjectBuilder`.
    pub fn new() -> ObjectBuilder {
        ObjectBuilder { object: Map::new() }
    }

    /// Return the constructed `Value`.
    pub fn unwrap(self) -> Value {
        Value::Object(self.object)
    }

    /// Insert a key-value pair into the object.
    pub fn insert<S, V>(mut self, key: S, value: V) -> ObjectBuilder
    where
        S: Into<String>,
        V: ser::Serialize,
    {
        self.object.insert(
            key.into(),
            value::to_value(&value).expect("failed to serialize"),
        );
        self
    }

    /// Creates and passes an `ObjectBuilder` into a closure, then inserts the resulting array into
    /// this object.
    pub fn insert_array<S, F>(mut self, key: S, f: F) -> ObjectBuilder
    where
        S: Into<String>,
        F: FnOnce(ArrayBuilder) -> ArrayBuilder,
    {
        let builder = ArrayBuilder::new();
        self.object.insert(key.into(), f(builder).unwrap());
        self
    }

    /// Creates and passes an `ObjectBuilder` into a closure, then inserts the resulting object into
    /// this object.
    pub fn insert_object<S, F>(mut self, key: S, f: F) -> ObjectBuilder
    where
        S: Into<String>,
        F: FnOnce(ObjectBuilder) -> ObjectBuilder,
    {
        let builder = ObjectBuilder::new();
        self.object.insert(key.into(), f(builder).unwrap());
        self
    }
}
