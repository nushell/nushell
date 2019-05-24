use crate::prelude::*;

use crate::object::{DataDescriptor, DescriptorName};
use crate::object::{Primitive, Value};
use indexmap::IndexMap;
use std::cmp::{Ordering, PartialOrd};

#[derive(Debug, Default, Eq, PartialEq)]
pub struct Dictionary {
    entries: IndexMap<DataDescriptor, Value>,
}

impl PartialOrd for Dictionary {
    // TODO: FIXME
    fn partial_cmp(&self, _other: &Dictionary) -> Option<Ordering> {
        Some(Ordering::Less)
    }
}

impl Ord for Dictionary {
    // TODO: FIXME
    fn cmp(&self, _other: &Dictionary) -> Ordering {
        Ordering::Less
    }
}

impl PartialOrd<Value> for Dictionary {
    fn partial_cmp(&self, _other: &Value) -> Option<Ordering> {
        Some(Ordering::Less)
    }
}

impl PartialEq<Value> for Dictionary {
    // TODO: FIXME
    fn eq(&self, other: &Value) -> bool {
        match other {
            Value::Object(d) => self == d,
            _ => false,
        }
    }
}

impl Dictionary {
    crate fn add(&mut self, name: impl Into<DataDescriptor>, value: Value) {
        self.entries.insert(name.into(), value);
    }

    crate fn copy_dict(&self) -> Dictionary {
        let mut out = Dictionary::default();

        for (key, value) in self.entries.iter() {
            out.add(key.copy(), value.copy());
        }

        out
    }

    crate fn data_descriptors(&self) -> Vec<DataDescriptor> {
        self.entries.iter().map(|(name, _)| name.copy()).collect()
    }

    crate fn get_data(&'a self, desc: &DataDescriptor) -> MaybeOwned<'a, Value> {
        match self.entries.get(desc) {
            Some(v) => MaybeOwned::Borrowed(v),
            None => MaybeOwned::Owned(Value::Primitive(Primitive::Nothing)),
        }
    }

    crate fn get_data_by_key(&self, name: &str) -> MaybeOwned<'_, Value> {
        match self
            .entries
            .iter()
            .find(|(desc_name, _)| desc_name.name.is_string(name))
        {
            Some((_, v)) => MaybeOwned::Borrowed(v),
            None => MaybeOwned::Owned(Value::Primitive(Primitive::Nothing)),
        }
    }
}
