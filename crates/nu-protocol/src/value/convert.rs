use crate::type_name::SpannedTypeName;
use crate::value::dict::Dictionary;
use crate::value::primitive::Primitive;
use crate::value::{UntaggedValue, Value};
use nu_errors::{CoerceInto, ShellError};
use nu_source::TaggedItem;

impl std::convert::TryFrom<&Value> for i64 {
    type Error = ShellError;

    fn try_from(value: &Value) -> Result<i64, ShellError> {
        match &value.value {
            UntaggedValue::Primitive(Primitive::Int(int)) => {
                int.tagged(&value.tag).coerce_into("converting to i64")
            }
            _ => Err(ShellError::type_error("Integer", value.spanned_type_name())),
        }
    }
}

impl std::convert::TryFrom<&Value> for String {
    type Error = ShellError;

    fn try_from(value: &Value) -> Result<String, ShellError> {
        match &value.value {
            UntaggedValue::Primitive(Primitive::String(s)) => Ok(s.clone()),
            _ => Err(ShellError::type_error("String", value.spanned_type_name())),
        }
    }
}

impl std::convert::TryFrom<&Value> for Vec<u8> {
    type Error = ShellError;

    fn try_from(value: &Value) -> Result<Vec<u8>, ShellError> {
        match &value.value {
            UntaggedValue::Primitive(Primitive::Binary(b)) => Ok(b.clone()),
            _ => Err(ShellError::type_error("Binary", value.spanned_type_name())),
        }
    }
}

impl<'a> std::convert::TryFrom<&'a Value> for &'a Dictionary {
    type Error = ShellError;

    fn try_from(value: &'a Value) -> Result<&'a Dictionary, ShellError> {
        match &value.value {
            UntaggedValue::Row(d) => Ok(d),
            _ => Err(ShellError::type_error(
                "Dictionary",
                value.spanned_type_name(),
            )),
        }
    }
}
