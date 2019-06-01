use crate::prelude::*;

use crate::object::DataDescriptor;
use crate::object::{Primitive, Value};
use derive_new::new;
use indexmap::IndexMap;
use serde_derive::Deserialize;
use serde::ser::{Serialize, Serializer, SerializeMap};
use std::cmp::{Ordering, PartialOrd};

#[derive(Debug, Default, Eq, PartialEq, Deserialize, Clone, new)]
pub struct Dictionary {
    entries: IndexMap<DataDescriptor, Value>,
}

impl PartialOrd for Dictionary {
    // TODO: FIXME
    fn partial_cmp(&self, _other: &Dictionary) -> Option<Ordering> {
        Some(Ordering::Less)
    }
}

impl Serialize for Dictionary {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.entries.len()))?;
        for (k, v) in self.entries.iter() {
            match v {
                Value::Object(_) => {},
                _ => map.serialize_entry(k, v)?
            }
        }
        for (k, v) in self.entries.iter() {
            match v {
                Value::Object(_) => map.serialize_entry(k, v)?,
                _ => {}
            }
        }
        map.end()
    }
}

impl From<IndexMap<String, Value>> for Dictionary {
    fn from(input: IndexMap<String, Value>) -> Dictionary {
        let mut out = IndexMap::default();

        for (key, value) in input {
            out.insert(DataDescriptor::for_string_name(key), value);
        }

        Dictionary::new(out)
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

    crate fn get_data_by_key(&self, name: &str) -> Option<&Value> {
        match self
            .entries
            .iter()
            .find(|(desc_name, _)| desc_name.name.is_string(name))
        {
            Some((_, v)) => Some(v),
            None => None,
        }
    }
}
