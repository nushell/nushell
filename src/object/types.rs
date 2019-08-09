use crate::object::base as value;
use crate::parser::hir;
use crate::prelude::*;
use log::trace;
use std::path::PathBuf;

pub trait ExtractType: Sized {
    fn extract(value: &Tagged<Value>) -> Result<Self, ShellError>;
    fn check(value: &'value Tagged<Value>) -> Result<&'value Tagged<Value>, ShellError>;
    fn syntax_type() -> hir::SyntaxType {
        hir::SyntaxType::Any
    }
}

impl<T> ExtractType for T {
    default fn extract(_value: &Tagged<Value>) -> Result<T, ShellError> {
        let name = std::intrinsics::type_name::<T>();
        Err(ShellError::unimplemented(format!(
            "<T> ExtractType for {}",
            name
        )))
    }

    default fn check(_value: &'value Tagged<Value>) -> Result<&'value Tagged<Value>, ShellError> {
        Err(ShellError::unimplemented("ExtractType for T"))
    }

    default fn syntax_type() -> hir::SyntaxType {
        hir::SyntaxType::Any
    }
}

impl<T: ExtractType> ExtractType for Vec<Tagged<T>> {
    fn extract(value: &Tagged<Value>) -> Result<Self, ShellError> {
        let name = std::intrinsics::type_name::<T>();
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

    fn check(value: &'value Tagged<Value>) -> Result<&'value Tagged<Value>, ShellError> {
        match value.item() {
            Value::List(_) => Ok(value),
            other => Err(ShellError::type_error(
                "Vec",
                other.type_name().tagged(value.tag()),
            )),
        }
    }

    fn syntax_type() -> hir::SyntaxType {
        hir::SyntaxType::List
    }
}

impl<T: ExtractType, U: ExtractType> ExtractType for (T, U) {
    fn extract(value: &Tagged<Value>) -> Result<(T, U), ShellError> {
        let t_name = std::intrinsics::type_name::<T>();
        let u_name = std::intrinsics::type_name::<U>();

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
        let name = std::intrinsics::type_name::<T>();
        trace!("<Option> Extracting {:?} for Option<{}>", value, name);

        let result = match value.item() {
            Value::Primitive(Primitive::Nothing) => None,
            _ => Some(T::extract(value)?),
        };

        Ok(result)
    }

    fn check(value: &'value Tagged<Value>) -> Result<&'value Tagged<Value>, ShellError> {
        match value.item() {
            Value::Primitive(Primitive::Nothing) => Ok(value),
            _ => T::check(value),
        }
    }

    fn syntax_type() -> hir::SyntaxType {
        T::syntax_type()
    }
}

impl<T: ExtractType> ExtractType for Tagged<T> {
    fn extract(value: &Tagged<Value>) -> Result<Tagged<T>, ShellError> {
        let name = std::intrinsics::type_name::<T>();
        trace!("<Tagged> Extracting {:?} for Tagged<{}>", value, name);

        Ok(T::extract(value)?.tagged(value.tag()))
    }

    fn check(value: &'value Tagged<Value>) -> Result<&'value Tagged<Value>, ShellError> {
        T::check(value)
    }

    fn syntax_type() -> hir::SyntaxType {
        T::syntax_type()
    }
}

impl ExtractType for Value {
    fn extract(value: &Tagged<Value>) -> Result<Value, ShellError> {
        trace!("<Tagged> Extracting {:?} for Value", value);

        Ok(value.item().clone())
    }

    fn check(value: &'value Tagged<Value>) -> Result<&'value Tagged<Value>, ShellError> {
        Ok(value)
    }

    fn syntax_type() -> hir::SyntaxType {
        SyntaxType::Any
    }
}

impl ExtractType for bool {
    fn syntax_type() -> hir::SyntaxType {
        hir::SyntaxType::Boolean
    }

    fn extract(value: &'a Tagged<Value>) -> Result<bool, ShellError> {
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

    fn check(value: &'value Tagged<Value>) -> Result<&'value Tagged<Value>, ShellError> {
        match &value {
            value @ Tagged {
                item: Value::Primitive(Primitive::Boolean(_)),
                ..
            } => Ok(value),
            other => Err(ShellError::type_error("Boolean", other.tagged_type_name())),
        }
    }
}

impl ExtractType for std::path::PathBuf {
    fn syntax_type() -> hir::SyntaxType {
        hir::SyntaxType::Path
    }

    fn extract(value: &'a Tagged<Value>) -> Result<std::path::PathBuf, ShellError> {
        trace!("Extracting {:?} for PathBuf", value);

        match &value {
            Tagged {
                item: Value::Primitive(Primitive::String(p)),
                ..
            } => Ok(PathBuf::from(p)),
            other => Err(ShellError::type_error("Path", other.tagged_type_name())),
        }
    }

    fn check(value: &'value Tagged<Value>) -> Result<&'value Tagged<Value>, ShellError> {
        match &value {
            v @ Tagged {
                item: Value::Primitive(Primitive::Path(_)),
                ..
            } => Ok(v),
            other => Err(ShellError::type_error("Path", other.tagged_type_name())),
        }
    }
}

impl ExtractType for i64 {
    fn extract(value: &Tagged<Value>) -> Result<i64, ShellError> {
        trace!("Extracting {:?} for i64", value);

        match value {
            &Tagged {
                item: Value::Primitive(Primitive::Int(int)),
                ..
            } => Ok(int),
            other => Err(ShellError::type_error("Integer", other.tagged_type_name())),
        }
    }

    fn check(value: &'value Tagged<Value>) -> Result<&'value Tagged<Value>, ShellError> {
        match value {
            v @ Tagged {
                item: Value::Primitive(Primitive::Int(_)),
                ..
            } => Ok(v),
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

    fn check(value: &'value Tagged<Value>) -> Result<&'value Tagged<Value>, ShellError> {
        match value {
            v @ Tagged {
                item: Value::Primitive(Primitive::String(_)),
                ..
            } => Ok(v),
            other => Err(ShellError::type_error("String", other.tagged_type_name())),
        }
    }
}

impl ExtractType for value::Block {
    fn check(value: &'value Tagged<Value>) -> Result<&'value Tagged<Value>, ShellError> {
        trace!("Extracting {:?} for Block", value);

        match value {
            v @ Tagged {
                item: Value::Block(_),
                ..
            } => Ok(v),
            other => Err(ShellError::type_error("Block", other.tagged_type_name())),
        }
    }

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
