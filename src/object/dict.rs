use crate::prelude::*;

use crate::object::{Primitive, Value};
use derive_new::new;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::cmp::{Ordering, PartialOrd};
use std::fmt;

#[derive(Debug, Default, Eq, PartialEq, Serialize, Deserialize, Clone, new)]
pub struct Dictionary {
    pub entries: IndexMap<String, Tagged<Value>>,
}

impl PartialOrd for Dictionary {
    fn partial_cmp(&self, other: &Dictionary) -> Option<Ordering> {
        let this: Vec<&String> = self.entries.keys().collect();
        let that: Vec<&String> = other.entries.keys().collect();

        if this != that {
            return this.partial_cmp(&that);
        }

        let this: Vec<&Value> = self.entries.values().map(|v| v.item()).collect();
        let that: Vec<&Value> = self.entries.values().map(|v| v.item()).collect();

        this.partial_cmp(&that)
    }
}

impl From<IndexMap<String, Tagged<Value>>> for Dictionary {
    fn from(input: IndexMap<String, Tagged<Value>>) -> Dictionary {
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

        let this: Vec<&Value> = self.entries.values().map(|v| v.item()).collect();
        let that: Vec<&Value> = self.entries.values().map(|v| v.item()).collect();

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
    pub fn get_data(&'a self, desc: &String) -> MaybeOwned<'a, Value> {
        match self.entries.get(desc) {
            Some(v) => MaybeOwned::Borrowed(v),
            None => MaybeOwned::Owned(Value::Primitive(Primitive::Nothing)),
        }
    }

    crate fn get_data_by_key(&self, name: &str) -> Option<&Tagged<Value>> {
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

pub struct TaggedListBuilder {
    span: Span,
    list: Vec<Tagged<Value>>,
}

impl TaggedListBuilder {
    pub fn new(span: impl Into<Span>) -> TaggedListBuilder {
        TaggedListBuilder {
            span: span.into(),
            list: vec![],
        }
    }

    pub fn push(&mut self, value: impl Into<Value>) {
        self.list.push(value.into().tagged(self.span));
    }

    pub fn insert_tagged(&mut self, value: impl Into<Tagged<Value>>) {
        self.list.push(value.into());
    }

    pub fn into_tagged_value(self) -> Tagged<Value> {
        Value::List(self.list).tagged(self.span)
    }
}

impl From<TaggedListBuilder> for Tagged<Value> {
    fn from(input: TaggedListBuilder) -> Tagged<Value> {
        input.into_tagged_value()
    }
}

#[derive(Debug)]
pub struct TaggedDictBuilder {
    span: Span,
    dict: IndexMap<String, Tagged<Value>>,
}

impl TaggedDictBuilder {
    pub fn new(span: impl Into<Span>) -> TaggedDictBuilder {
        TaggedDictBuilder {
            span: span.into(),
            dict: IndexMap::default(),
        }
    }

    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<Value>) {
        self.dict.insert(key.into(), value.into().tagged(self.span));
    }

    pub fn insert_tagged(&mut self, key: impl Into<String>, value: impl Into<Tagged<Value>>) {
        self.dict.insert(key.into(), value.into());
    }

    pub fn into_tagged_value(self) -> Tagged<Value> {
        self.into_tagged_dict().map(Value::Object)
    }

    pub fn into_tagged_dict(self) -> Tagged<Dictionary> {
        Dictionary { entries: self.dict }.tagged(self.span)
    }
}

impl From<TaggedDictBuilder> for Tagged<Value> {
    fn from(input: TaggedDictBuilder) -> Tagged<Value> {
        input.into_tagged_value()
    }
}
