use crate::maybe_owned::MaybeOwned;
use crate::value::primitive::Primitive;
use crate::value::{UntaggedValue, Value};
use derive_new::new;
use getset::Getters;
use indexmap::IndexMap;
use nu_source::{b, DebugDocBuilder, PrettyDebug, Spanned, Tag};
use serde::{Deserialize, Serialize};
use std::cmp::{Ord, Ordering, PartialOrd};
use std::hash::{Hash, Hasher};

#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Eq, Clone, Getters, new)]
pub struct Dictionary {
    #[get = "pub"]
    pub entries: IndexMap<String, Value>,
}

#[allow(clippy::derive_hash_xor_eq)]
impl Hash for Dictionary {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let mut entries = self.entries.clone();
        entries.sort_keys();
        entries.keys().collect::<Vec<&String>>().hash(state);
        entries.values().collect::<Vec<&Value>>().hash(state);
    }
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

impl PartialEq<Value> for Dictionary {
    fn eq(&self, other: &Value) -> bool {
        match &other.value {
            UntaggedValue::Row(d) => self == d,
            _ => false,
        }
    }
}

#[derive(Debug, new)]
struct DebugEntry<'a> {
    key: &'a str,
    value: &'a Value,
}

impl<'a> PrettyDebug for DebugEntry<'a> {
    fn pretty(&self) -> DebugDocBuilder {
        (b::key(self.key.to_string()) + b::equals() + self.value.pretty().into_value()).group()
    }
}

impl PrettyDebug for Dictionary {
    fn pretty(&self) -> DebugDocBuilder {
        b::delimit(
            "(",
            b::intersperse(
                self.entries()
                    .iter()
                    .map(|(key, value)| DebugEntry::new(key, value)),
                b::space(),
            ),
            ")",
        )
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

impl Dictionary {
    pub fn get_data(&self, desc: &str) -> MaybeOwned<'_, Value> {
        match self.entries.get(desc) {
            Some(v) => MaybeOwned::Borrowed(v),
            None => MaybeOwned::Owned(
                UntaggedValue::Primitive(Primitive::Nothing).into_untagged_value(),
            ),
        }
    }

    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.entries.keys()
    }

    pub fn get_data_by_key(&self, name: Spanned<&str>) -> Option<Value> {
        let result = self
            .entries
            .iter()
            .find(|(desc_name, _)| *desc_name == name.item)?
            .1;

        Some(
            result
                .value
                .clone()
                .into_value(Tag::new(result.tag.anchor(), name.span)),
        )
    }

    pub fn get_mut_data_by_key(&mut self, name: &str) -> Option<&mut Value> {
        match self
            .entries
            .iter_mut()
            .find(|(desc_name, _)| *desc_name == name)
        {
            Some((_, v)) => Some(v),
            None => None,
        }
    }

    pub fn insert_data_at_key(&mut self, name: &str, value: Value) {
        self.entries.insert(name.to_string(), value);
    }
}

#[derive(Debug)]
pub struct TaggedDictBuilder {
    tag: Tag,
    dict: IndexMap<String, Value>,
}

impl TaggedDictBuilder {
    pub fn new(tag: impl Into<Tag>) -> TaggedDictBuilder {
        TaggedDictBuilder {
            tag: tag.into(),
            dict: IndexMap::default(),
        }
    }

    pub fn build(tag: impl Into<Tag>, block: impl FnOnce(&mut TaggedDictBuilder)) -> Value {
        let mut builder = TaggedDictBuilder::new(tag);
        block(&mut builder);
        builder.into_value()
    }

    pub fn with_capacity(tag: impl Into<Tag>, n: usize) -> TaggedDictBuilder {
        TaggedDictBuilder {
            tag: tag.into(),
            dict: IndexMap::with_capacity(n),
        }
    }

    pub fn insert_untagged(&mut self, key: impl Into<String>, value: impl Into<UntaggedValue>) {
        self.dict
            .insert(key.into(), value.into().into_value(&self.tag));
    }

    pub fn insert_value(&mut self, key: impl Into<String>, value: impl Into<Value>) {
        self.dict.insert(key.into(), value.into());
    }

    pub fn into_value(self) -> Value {
        let tag = self.tag.clone();
        self.into_untagged_value().into_value(tag)
    }

    pub fn into_untagged_value(self) -> UntaggedValue {
        UntaggedValue::Row(Dictionary { entries: self.dict })
    }

    pub fn is_empty(&self) -> bool {
        self.dict.is_empty()
    }
}

impl From<TaggedDictBuilder> for Value {
    fn from(input: TaggedDictBuilder) -> Value {
        input.into_value()
    }
}
