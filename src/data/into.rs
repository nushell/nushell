use crate::data::base::{Primitive, UntaggedValue};

impl From<Primitive> for UntaggedValue {
    fn from(input: Primitive) -> UntaggedValue {
        UntaggedValue::Primitive(input)
    }
}

impl From<String> for UntaggedValue {
    fn from(input: String) -> UntaggedValue {
        UntaggedValue::Primitive(Primitive::String(input))
    }
}
