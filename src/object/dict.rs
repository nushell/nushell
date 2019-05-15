#[allow(unused)]
use crate::prelude::*;

use crate::object::desc::DataDescriptor;
use crate::object::{Primitive, Value};
use crate::MaybeOwned;
use indexmap::IndexMap;

#[derive(Debug, Default)]
pub struct Dictionary {
    entries: IndexMap<String, Value>,
}

impl Dictionary {
    crate fn add(&mut self, name: impl Into<String>, value: Value) {
        self.entries.insert(name.into(), value);
    }

    crate fn copy_dict(&self) -> Dictionary {
        let mut out = Dictionary::default();

        for (key, value) in self.entries.iter() {
            out.add(key.clone(), value.copy());
        }

        out
    }

    crate fn data_descriptors(&self) -> Vec<DataDescriptor> {
        self.entries
            .iter()
            .map(|(name, _)| {
                DataDescriptor::new(name.clone(), true, Box::new(crate::object::types::AnyShell))
            })
            .collect()
    }

    crate fn get_data(&'a self, desc: &DataDescriptor) -> MaybeOwned<'a, Value> {
        match self.entries.get(&desc.name) {
            Some(v) => MaybeOwned::Borrowed(v),
            None => MaybeOwned::Owned(Value::Primitive(Primitive::Nothing)),
        }
    }
}
