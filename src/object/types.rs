use crate::object::base as value;
use crate::parser::hir;
use crate::prelude::*;
use log::trace;
use std::path::PathBuf;

pub trait ExtractType: Sized {
    fn extract(value: &Spanned<Value>) -> Result<Self, ShellError>;
    fn check(value: &'value Spanned<Value>) -> Result<&'value Spanned<Value>, ShellError>;
    fn syntax_type() -> hir::SyntaxType {
        hir::SyntaxType::Any
    }
}

impl<T> ExtractType for T {
    default fn extract(_value: &Spanned<Value>) -> Result<T, ShellError> {
        let name = std::intrinsics::type_name::<T>();
        Err(ShellError::unimplemented(format!(
            "<T> ExtractType for {}",
            name
        )))
    }

    default fn check(_value: &'value Spanned<Value>) -> Result<&'value Spanned<Value>, ShellError> {
        Err(ShellError::unimplemented("ExtractType for T"))
    }

    default fn syntax_type() -> hir::SyntaxType {
        hir::SyntaxType::Any
    }
}

impl<T: ExtractType> ExtractType for Vec<Spanned<T>> {
    fn extract(value: &Spanned<Value>) -> Result<Self, ShellError> {
        let name = std::intrinsics::type_name::<T>();
        trace!("<Vec> Extracting {:?} for Vec<{}>", value, name);

        match value.item() {
            Value::List(items) => {
                let mut out = vec![];

                for item in items {
                    out.push(T::extract(item)?.spanned(item.span));
                }

                Ok(out)
            }
            other => Err(ShellError::type_error(
                "Vec",
                other.type_name().spanned(value.span),
            )),
        }
    }

    fn check(value: &'value Spanned<Value>) -> Result<&'value Spanned<Value>, ShellError> {
        match value.item() {
            Value::List(_) => Ok(value),
            other => Err(ShellError::type_error(
                "Vec",
                other.type_name().spanned(value.span),
            )),
        }
    }

    fn syntax_type() -> hir::SyntaxType {
        hir::SyntaxType::List
    }
}

impl<T: ExtractType, U: ExtractType> ExtractType for (T, U) {
    fn extract(value: &Spanned<Value>) -> Result<(T, U), ShellError> {
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
                        "not-two".spanned(value.span),
                    ))
                }
            }
            other => Err(ShellError::type_error(
                "two-element-tuple",
                other.type_name().spanned(value.span),
            )),
        }
    }
}

impl<T: ExtractType> ExtractType for Option<T> {
    fn extract(value: &Spanned<Value>) -> Result<Option<T>, ShellError> {
        let name = std::intrinsics::type_name::<T>();
        trace!("<Option> Extracting {:?} for Option<{}>", value, name);

        let result = match value.item() {
            Value::Primitive(Primitive::Nothing) => None,
            _ => Some(T::extract(value)?),
        };

        Ok(result)
    }

    fn check(value: &'value Spanned<Value>) -> Result<&'value Spanned<Value>, ShellError> {
        match value.item() {
            Value::Primitive(Primitive::Nothing) => Ok(value),
            _ => T::check(value),
        }
    }

    fn syntax_type() -> hir::SyntaxType {
        T::syntax_type()
    }
}

impl<T: ExtractType> ExtractType for Spanned<T> {
    fn extract(value: &Spanned<Value>) -> Result<Spanned<T>, ShellError> {
        let name = std::intrinsics::type_name::<T>();
        trace!("<Spanned> Extracting {:?} for Spanned<{}>", value, name);

        Ok(T::extract(value)?.spanned(value.span))
    }

    fn check(value: &'value Spanned<Value>) -> Result<&'value Spanned<Value>, ShellError> {
        T::check(value)
    }

    fn syntax_type() -> hir::SyntaxType {
        T::syntax_type()
    }
}

impl ExtractType for Value {
    fn extract(value: &Spanned<Value>) -> Result<Value, ShellError> {
        trace!("<Spanned> Extracting {:?} for Value", value);

        Ok(value.item().clone())
    }

    fn check(value: &'value Spanned<Value>) -> Result<&'value Spanned<Value>, ShellError> {
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

    fn extract(value: &'a Spanned<Value>) -> Result<bool, ShellError> {
        trace!("Extracting {:?} for bool", value);

        match &value {
            Spanned {
                item: Value::Primitive(Primitive::Boolean(b)),
                ..
            } => Ok(*b),
            Spanned {
                item: Value::Primitive(Primitive::Nothing),
                ..
            } => Ok(false),
            other => Err(ShellError::type_error("Boolean", other.spanned_type_name())),
        }
    }

    fn check(value: &'value Spanned<Value>) -> Result<&'value Spanned<Value>, ShellError> {
        match &value {
            value @ Spanned {
                item: Value::Primitive(Primitive::Boolean(_)),
                ..
            } => Ok(value),
            other => Err(ShellError::type_error("Boolean", other.spanned_type_name())),
        }
    }
}

impl ExtractType for std::path::PathBuf {
    fn syntax_type() -> hir::SyntaxType {
        hir::SyntaxType::Path
    }

    fn extract(value: &'a Spanned<Value>) -> Result<std::path::PathBuf, ShellError> {
        trace!("Extracting {:?} for PathBuf", value);

        match &value {
            Spanned {
                item: Value::Primitive(Primitive::String(p)),
                ..
            } => Ok(PathBuf::from(p)),
            other => Err(ShellError::type_error("Path", other.spanned_type_name())),
        }
    }

    fn check(value: &'value Spanned<Value>) -> Result<&'value Spanned<Value>, ShellError> {
        match &value {
            v @ Spanned {
                item: Value::Primitive(Primitive::Path(_)),
                ..
            } => Ok(v),
            other => Err(ShellError::type_error("Path", other.spanned_type_name())),
        }
    }
}

impl ExtractType for i64 {
    fn extract(value: &Spanned<Value>) -> Result<i64, ShellError> {
        trace!("Extracting {:?} for i64", value);

        match value {
            &Spanned {
                item: Value::Primitive(Primitive::Int(int)),
                ..
            } => Ok(int),
            other => Err(ShellError::type_error("Integer", other.spanned_type_name())),
        }
    }

    fn check(value: &'value Spanned<Value>) -> Result<&'value Spanned<Value>, ShellError> {
        match value {
            v @ Spanned {
                item: Value::Primitive(Primitive::Int(_)),
                ..
            } => Ok(v),
            other => Err(ShellError::type_error("Integer", other.spanned_type_name())),
        }
    }
}

impl ExtractType for String {
    fn extract(value: &Spanned<Value>) -> Result<String, ShellError> {
        trace!("Extracting {:?} for String", value);

        match value {
            Spanned {
                item: Value::Primitive(Primitive::String(string)),
                ..
            } => Ok(string.clone()),
            other => Err(ShellError::type_error("String", other.spanned_type_name())),
        }
    }

    fn check(value: &'value Spanned<Value>) -> Result<&'value Spanned<Value>, ShellError> {
        match value {
            v @ Spanned {
                item: Value::Primitive(Primitive::String(_)),
                ..
            } => Ok(v),
            other => Err(ShellError::type_error("String", other.spanned_type_name())),
        }
    }
}

impl ExtractType for value::Block {
    fn check(value: &'value Spanned<Value>) -> Result<&'value Spanned<Value>, ShellError> {
        trace!("Extracting {:?} for Block", value);

        match value {
            v @ Spanned {
                item: Value::Block(_),
                ..
            } => Ok(v),
            other => Err(ShellError::type_error("Block", other.spanned_type_name())),
        }
    }

    fn extract(value: &Spanned<Value>) -> Result<value::Block, ShellError> {
        match value {
            Spanned {
                item: Value::Block(block),
                ..
            } => Ok(block.clone()),
            other => Err(ShellError::type_error("Block", other.spanned_type_name())),
        }
    }
}
