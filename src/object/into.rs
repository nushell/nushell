use crate::object::{Primitive, Value};
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

impl<T: Into<Value>> Spanned<T> {
    pub fn into_spanned_value(self) -> Spanned<Value> {
        let Spanned { item, span } = self;

        let value = item.into();
        value.spanned(span)
    }
}
