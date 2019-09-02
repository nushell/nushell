use crate::object::base as value;
use crate::prelude::*;
use log::trace;

pub trait ExtractType: Sized {
    fn extract(value: &Tagged<Value>) -> Result<Self, ShellError>;
}

impl<T> ExtractType for T {
    default fn extract(_value: &Tagged<Value>) -> Result<T, ShellError> {
        let name = std::any::type_name::<T>();
        Err(ShellError::unimplemented(format!(
            "<T> ExtractType for {}",
            name
        )))
    }
}

impl<T: ExtractType> ExtractType for Vec<Tagged<T>> {
    fn extract(value: &Tagged<Value>) -> Result<Self, ShellError> {
        let name = std::any::type_name::<T>();
        trace!("<Vec> Extracting {:?} for Vec<{}>", value, name);

        match value.item() {
            Value::List(items) => {
                let mut out = vec![];

                for item in items {
                    out.push(T::extract(item)?.tagged(item.tag()));
                }

                Ok(out)
            }
            other => Err(ShellError::type_error(
                "Vec",
                other.type_name().tagged(value.tag()),
            )),
        }
    }
}

impl<T: ExtractType, U: ExtractType> ExtractType for (T, U) {
    fn extract(value: &Tagged<Value>) -> Result<(T, U), ShellError> {
        let t_name = std::any::type_name::<T>();
        let u_name = std::any::type_name::<U>();

        trace!("Extracting {:?} for ({}, {})", value, t_name, u_name);

        match value.item() {
            Value::List(items) => {
                if items.len() == 2 {
                    let first = &items[0];
                    let second = &items[1];

                    Ok((T::extract(first)?, U::extract(second)?))
                } else {
                    Err(ShellError::type_error(
                        "two-element-tuple",
                        "not-two".tagged(value.tag()),
                    ))
                }
            }
            other => Err(ShellError::type_error(
                "two-element-tuple",
                other.type_name().tagged(value.tag()),
            )),
        }
    }
}

impl<T: ExtractType> ExtractType for Option<T> {
    fn extract(value: &Tagged<Value>) -> Result<Option<T>, ShellError> {
        let name = std::any::type_name::<T>();
        trace!("<Option> Extracting {:?} for Option<{}>", value, name);

        let result = match value.item() {
            Value::Primitive(Primitive::Nothing) => None,
            _ => Some(T::extract(value)?),
        };

        Ok(result)
    }
}

impl<T: ExtractType> ExtractType for Tagged<T> {
    fn extract(value: &Tagged<Value>) -> Result<Tagged<T>, ShellError> {
        let name = std::any::type_name::<T>();
        trace!("<Tagged> Extracting {:?} for Tagged<{}>", value, name);

        Ok(T::extract(value)?.tagged(value.tag()))
    }
}

impl ExtractType for Value {
    fn extract(value: &Tagged<Value>) -> Result<Value, ShellError> {
        trace!("<Tagged> Extracting {:?} for Value", value);

        Ok(value.item().clone())
    }
}

impl ExtractType for bool {
    fn extract(value: &Tagged<Value>) -> Result<bool, ShellError> {
        trace!("Extracting {:?} for bool", value);

        match &value {
            Tagged {
                item: Value::Primitive(Primitive::Boolean(b)),
                ..
            } => Ok(*b),
            Tagged {
                item: Value::Primitive(Primitive::Nothing),
                ..
            } => Ok(false),
            other => Err(ShellError::type_error("Boolean", other.tagged_type_name())),
        }
    }
}

impl ExtractType for std::path::PathBuf {
    fn extract(value: &Tagged<Value>) -> Result<std::path::PathBuf, ShellError> {
        trace!("Extracting {:?} for PathBuf", value);

        match &value {
            Tagged {
                item: Value::Primitive(Primitive::Path(p)),
                ..
            } => Ok(p.clone()),
            other => Err(ShellError::type_error("Path", other.tagged_type_name())),
        }
    }
}

impl ExtractType for i64 {
    fn extract(value: &Tagged<Value>) -> Result<i64, ShellError> {
        trace!("Extracting {:?} for i64", value);

        match &value {
            &Tagged {
                item: Value::Primitive(Primitive::Int(int)),
                ..
            } => Ok(int.tagged(value.tag).coerce_into("converting to i64")?),
            other => Err(ShellError::type_error("Integer", other.tagged_type_name())),
        }
    }
}

impl ExtractType for u64 {
    fn extract(value: &Tagged<Value>) -> Result<u64, ShellError> {
        trace!("Extracting {:?} for u64", value);

        match &value {
            &Tagged {
                item: Value::Primitive(Primitive::Int(int)),
                ..
            } => Ok(int.tagged(value.tag).coerce_into("converting to u64")?),
            other => Err(ShellError::type_error("Integer", other.tagged_type_name())),
        }
    }
}

impl ExtractType for String {
    fn extract(value: &Tagged<Value>) -> Result<String, ShellError> {
        trace!("Extracting {:?} for String", value);

        match value {
            Tagged {
                item: Value::Primitive(Primitive::String(string)),
                ..
            } => Ok(string.clone()),
            other => Err(ShellError::type_error("String", other.tagged_type_name())),
        }
    }
}

impl ExtractType for value::Block {
    fn extract(value: &Tagged<Value>) -> Result<value::Block, ShellError> {
        match value {
            Tagged {
                item: Value::Block(block),
                ..
            } => Ok(block.clone()),
            other => Err(ShellError::type_error("Block", other.tagged_type_name())),
        }
    }
}
