use crate::errors::ShellError;
use crate::format::{EntriesView, GenericView};
use crate::object::desc::DataDescriptor;
use chrono::NaiveDateTime;
use std::fmt::Debug;

#[derive(Debug)]
pub enum Primitive {
    Nothing,
    Int(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    Date(NaiveDateTime),
}

impl Primitive {
    crate fn format(&self) -> String {
        match self {
            Primitive::Nothing => format!("Nothing"),
            Primitive::Int(i) => format!("{}", i),
            Primitive::Float(f) => format!("{}", f),
            Primitive::String(s) => format!("{:?}", s),
            Primitive::Boolean(b) => format!("{:?}", b),
            Primitive::Date(d) => format!("{}", d),
        }
    }
}

#[derive(Debug)]
pub enum Value {
    Primitive(Primitive),
    Object(Box<dyn ShellObject>),
    List(Vec<Value>),
}

impl ShellObject for Value {
    fn to_shell_string(&self) -> String {
        match self {
            Value::Primitive(p) => p.format(),
            Value::Object(o) => o.to_shell_string(),
            Value::List(l) => format!("[list List]"),
        }
    }

    fn data_descriptors(&self) -> Vec<DataDescriptor> {
        match self {
            Value::Primitive(p) => vec![],
            Value::Object(o) => o.data_descriptors(),
            Value::List(l) => vec![],
        }
    }

    fn get_data(&'a self, desc: &DataDescriptor) -> crate::MaybeOwned<'a, Value> {
        match self {
            Value::Primitive(p) => crate::MaybeOwned::Owned(Value::nothing()),
            Value::Object(o) => o.get_data(desc),
            Value::List(l) => crate::MaybeOwned::Owned(Value::nothing()),
        }
    }
}

impl Value {
    crate fn format_leaf(&self) -> String {
        match self {
            Value::Primitive(p) => p.format(),
            Value::Object(o) => format!("[object Object]"),
            Value::List(l) => format!("[list List]"),
        }
    }

    crate fn as_string(&self) -> Result<String, ShellError> {
        match self {
            Value::Primitive(Primitive::String(s)) => Ok(s.to_string()),

            // TODO: this should definitely be more general with better errors
            other => Err(ShellError::string(format!(
                "Expected string, got {:?}",
                other
            ))),
        }
    }

    crate fn as_int(&self) -> Result<i64, ShellError> {
        match self {
            Value::Primitive(Primitive::Int(i)) => Ok(*i),
            // TODO: this should definitely be more general with better errors
            other => Err(ShellError::string(format!(
                "Expected integer, got {:?}",
                other
            ))),
        }
    }

    crate fn string(s: impl Into<String>) -> Value {
        Value::Primitive(Primitive::String(s.into()))
    }

    crate fn int(s: impl Into<i64>) -> Value {
        Value::Primitive(Primitive::Int(s.into()))
    }

    crate fn boolean(s: impl Into<bool>) -> Value {
        Value::Primitive(Primitive::Boolean(s.into()))
    }

    crate fn nothing() -> Value {
        Value::Primitive(Primitive::Nothing)
    }

    crate fn list(values: impl Into<Vec<Value>>) -> Value {
        Value::List(values.into())
    }

    crate fn object(value: impl ShellObject + 'static) -> Value {
        Value::Object(Box::new(value))
    }
}

pub trait ShellObject: Debug {
    fn to_shell_string(&self) -> String;
    fn data_descriptors(&self) -> Vec<DataDescriptor>;
    fn get_data(&'a self, desc: &DataDescriptor) -> crate::MaybeOwned<'a, Value>;
}

pub trait ToEntriesView {
    fn to_entries_view(&self) -> EntriesView;
}

impl<T> ToEntriesView for T
where
    T: ShellObject,
{
    fn to_entries_view(&self) -> EntriesView {
        let descs = self.data_descriptors();
        let mut entries = vec![];

        for desc in descs {
            let value = self.get_data(&desc);

            let formatted_value = match value.borrow() {
                Value::Primitive(p) => p.format(),
                Value::Object(o) => format!("[object Object]"),
                Value::List(l) => format!("[object List]"),
            };

            entries.push((desc.name.clone(), formatted_value))
        }

        EntriesView::new(entries)
    }
}

impl ShellObject for Box<dyn ShellObject> {
    fn to_shell_string(&self) -> String {
        (**self).to_shell_string()
    }
    fn data_descriptors(&self) -> Vec<DataDescriptor> {
        (**self).data_descriptors()
    }

    fn get_data(&'a self, desc: &DataDescriptor) -> crate::MaybeOwned<'a, Value> {
        (**self).get_data(desc)
    }
}

pub trait ToGenericView {
    fn to_generic_view(&self) -> GenericView;
}

impl ToGenericView for Value {
    fn to_generic_view(&self) -> GenericView<'_> {
        GenericView::new(self)
    }
}
