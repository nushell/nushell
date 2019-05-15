use crate::object::desc::DataDescriptor;
use crate::object::{Primitive, Value};
use crate::MaybeOwned;
use derive_new::new;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

#[derive(Debug, Default)]
pub struct Dictionary {
    entries: BTreeMap<String, Value>,
}

impl Dictionary {
    crate fn add(&mut self, name: impl Into<String>, value: Value) {
        self.entries.insert(name.into(), value);
    }
}

impl crate::object::ShellObject for Dictionary {
    fn to_shell_string(&self) -> String {
        format!("[object Object] lol")
    }

    fn data_descriptors(&self) -> Vec<DataDescriptor> {
        self.entries
            .iter()
            .map(|(name, value)| {
                DataDescriptor::new(name.clone(), true, Box::new(crate::object::types::AnyShell))
            })
            .collect()
    }

    fn get_data(&'a self, desc: &DataDescriptor) -> MaybeOwned<'a, Value> {
        match self.entries.get(&desc.name) {
            Some(v) => MaybeOwned::Borrowed(v),
            None => MaybeOwned::Owned(Value::Primitive(Primitive::Nothing)),
        }
    }
}

#[derive(Debug, Default)]
pub struct ScopedDictionary<'parent> {
    entries: BTreeMap<String, MaybeOwned<'parent, Value>>,
}

impl ScopedDictionary<'parent> {
    crate fn add(&mut self, name: impl Into<String>, value: impl Into<MaybeOwned<'parent, Value>>) {
        self.entries.insert(name.into(), value.into());
    }
}

impl crate::object::ShellObject for ScopedDictionary<'parent> {
    fn to_shell_string(&self) -> String {
        format!("[object Object] lol")
    }

    fn data_descriptors(&self) -> Vec<DataDescriptor> {
        self.entries
            .iter()
            .map(|(name, value)| {
                DataDescriptor::new(name.clone(), true, Box::new(crate::object::types::AnyShell))
            })
            .collect()
    }

    fn get_data(&'a self, desc: &DataDescriptor) -> MaybeOwned<'a, Value> {
        match self.entries.get(&desc.name) {
            Some(v) => MaybeOwned::Borrowed(v.borrow()),
            None => MaybeOwned::Owned(Value::Primitive(Primitive::Nothing)),
        }
    }
}
