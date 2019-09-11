use crate::data::{Primitive, Value};
use crate::prelude::*;

impl From<Primitive> for Value {
    fn from(input: Primitive) -> Value {
        Value::Primitive(input)
    }
}

impl From<String> for Value {
    fn from(input: String) -> Value {
        Value::Primitive(Primitive::String(input))
    }
}

impl<T: Into<Value>> Tagged<T> {
    pub fn into_tagged_value(self) -> Tagged<Value> {
        let value_span = self.span();
        let value = self.item.into();
        value.simple_spanned(value_span)
    }
}
