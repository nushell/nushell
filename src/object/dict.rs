use crate::prelude::*;

use crate::object::{Primitive, Value};
use derive_new::new;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::cmp::{Ordering, PartialOrd};
use std::fmt;

#[derive(Debug, Default, Eq, PartialEq, Serialize, Deserialize, Clone, new)]
pub struct Dictionary {
    pub entries: IndexMap<String, Value>,
}

impl PartialOrd for Dictionary {
    fn partial_cmp(&self, other: &Dictionary) -> Option<Ordering> {
        let this: Vec<&String> = self.entries.keys().collect();
        let that: Vec<&String> = other.entries.keys().collect();

        if this != that {
            return this.partial_cmp(&that);
        }

        let this: Vec<&Value> = self.entries.values().collect();
        let that: Vec<&Value> = self.entries.values().collect();

        this.partial_cmp(&that)
    }
}

impl From<IndexMap<String, Value>> for Dictionary {
    fn from(input: IndexMap<String, Value>) -> Dictionary {
        let mut out = IndexMap::default();

        for (key, value) in input {
            out.insert(key, value);
        }

        Dictionary::new(out)
    }
}

impl Ord for Dictionary {
    fn cmp(&self, other: &Dictionary) -> Ordering {
        let this: Vec<&String> = self.entries.keys().collect();
        let that: Vec<&String> = other.entries.keys().collect();

        if this != that {
            return this.cmp(&that);
        }

        let this: Vec<&Value> = self.entries.values().collect();
        let that: Vec<&Value> = self.entries.values().collect();

        this.cmp(&that)
    }
}

impl PartialOrd<Value> for Dictionary {
    fn partial_cmp(&self, _other: &Value) -> Option<Ordering> {
        Some(Ordering::Less)
    }
}

impl PartialEq<Value> for Dictionary {
    fn eq(&self, other: &Value) -> bool {
        match other {
            Value::Object(d) => self == d,
            _ => false,
        }
    }
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

    pub fn get_data(&'a self, desc: &String) -> MaybeOwned<'a, Value> {
        match self.entries.get(desc) {
            Some(v) => MaybeOwned::Borrowed(v),
            None => MaybeOwned::Owned(Value::Primitive(Primitive::Nothing)),
        }
    }

    crate fn get_data_by_key(&self, name: &str) -> Option<&Value> {
        match self
            .entries
            .iter()
            .find(|(desc_name, _)| *desc_name == name)
        {
            Some((_, v)) => Some(v),
            None => None,
        }
    }

    crate fn debug(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut debug = f.debug_struct("Dictionary");

        for (desc, value) in self.entries.iter() {
            debug.field(desc, &value.debug());
        }

        debug.finish()
    }
}
