use crate::maybe_owned::MaybeOwned;
use crate::value::primitive::Primitive;
use crate::value::{UntaggedValue, Value};
use derive_new::new;
use getset::Getters;
use indexmap::IndexMap;
use nu_source::{DbgDocBldr, DebugDocBuilder, PrettyDebug, Spanned, SpannedItem, Tag};
use serde::{Deserialize, Serialize};
use std::cmp::{Ord, Ordering, PartialOrd};
use std::hash::{Hash, Hasher};

/// A dictionary that can hold a mapping from names to Values
#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Eq, Clone, Getters, new)]
pub struct Dictionary {
    #[get = "pub"]
    pub entries: IndexMap<String, Value>,
}

#[allow(clippy::derive_hash_xor_eq)]
impl Hash for Dictionary {
    /// Create the hash function to allow the Hash trait for dictionaries
    fn hash<H: Hasher>(&self, state: &mut H) {
        let mut entries = self.entries.clone();
        entries.sort_keys();
        entries.keys().collect::<Vec<&String>>().hash(state);
        entries.values().collect::<Vec<&Value>>().hash(state);
    }
}

impl PartialOrd for Dictionary {
    /// Compare two dictionaries for sort ordering
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
    /// Compare two dictionaries for ordering
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
    /// Test a dictionary against a Value for equality
    fn eq(&self, other: &Value) -> bool {
        matches!(&other.value, UntaggedValue::Row(d) if self == d)
    }
}

/// A key-value pair specifically meant to be used in debug and pretty-printing
#[derive(Debug, new)]
struct DebugEntry<'a> {
    key: &'a str,
    value: &'a Value,
}

impl<'a> PrettyDebug for DebugEntry<'a> {
    /// Build the the information to pretty-print the DebugEntry
    fn pretty(&self) -> DebugDocBuilder {
        (DbgDocBldr::key(self.key.to_string())
            + DbgDocBldr::equals()
            + self.value.pretty().into_value())
        .group()
    }
}

impl PrettyDebug for Dictionary {
    /// Get a Dictionary ready to be pretty-printed
    fn pretty(&self) -> DebugDocBuilder {
        DbgDocBldr::delimit(
            "(",
            DbgDocBldr::intersperse(
                self.entries()
                    .iter()
                    .map(|(key, value)| DebugEntry::new(key, value)),
                DbgDocBldr::space(),
            ),
            ")",
        )
    }
}

impl From<IndexMap<String, Value>> for Dictionary {
    /// Create a dictionary from a map of strings to Values
    fn from(input: IndexMap<String, Value>) -> Dictionary {
        let mut out = IndexMap::default();

        for (key, value) in input {
            out.insert(key, value);
        }

        Dictionary::new(out)
    }
}

impl Dictionary {
    /// Find the matching Value for a given key, if possible. If not, return a Primitive::Nothing
    pub fn get_data(&self, desc: &str) -> MaybeOwned<'_, Value> {
        match self.entries.get(desc) {
            Some(v) => MaybeOwned::Borrowed(v),
            None => MaybeOwned::Owned(
                UntaggedValue::Primitive(Primitive::Nothing).into_untagged_value(),
            ),
        }
    }

    pub fn insert(&mut self, key: String, value: Value) -> Option<Value> {
        self.entries.insert_full(key, value).1
    }

    pub fn merge_from(&self, other: &Dictionary) -> Dictionary {
        let mut obj = self.clone();

        for column in other.keys() {
            let key = column.clone();
            let value_key = key.as_str();
            let value_spanned_key = value_key.spanned_unknown();

            let other_column = match other.get_data_by_key(value_spanned_key) {
                Some(value) => value,
                None => UntaggedValue::Primitive(Primitive::Nothing).into_untagged_value(),
            };

            obj.entries.insert(key, other_column);
        }

        obj
    }

    /// Iterate the keys in the Dictionary
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.entries.keys()
    }

    /// Iterate the values in the Dictionary
    pub fn values(&self) -> impl Iterator<Item = &Value> {
        self.entries.values()
    }

    /// Checks if given key exists
    pub fn contains_key(&self, key: &str) -> bool {
        self.entries.contains_key(key)
    }

    /// Find the matching Value for a key, if possible
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

    /// Get a mutable entry that matches a key, if possible
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

    /// Insert a new key/value pair into the dictionary
    pub fn insert_data_at_key(&mut self, name: &str, value: Value) {
        self.entries.insert(name.to_string(), value);
    }

    /// Return size of dictionary
    pub fn length(&self) -> usize {
        self.entries.len()
    }
}

/// A helper to help create dictionaries for you. It has the ability to insert values into the dictionary while maintaining the tags that need to be applied to the individual members
#[derive(Debug, Clone)]
pub struct TaggedDictBuilder {
    tag: Tag,
    dict: IndexMap<String, Value>,
}

impl TaggedDictBuilder {
    /// Create a new builder
    pub fn new(tag: impl Into<Tag>) -> TaggedDictBuilder {
        TaggedDictBuilder {
            tag: tag.into(),
            dict: IndexMap::default(),
        }
    }

    /// Build the contents of the builder into a Value
    pub fn build(tag: impl Into<Tag>, block: impl FnOnce(&mut TaggedDictBuilder)) -> Value {
        let mut builder = TaggedDictBuilder::new(tag);
        block(&mut builder);
        builder.into_value()
    }

    /// Create a new builder with a pre-defined capacity
    pub fn with_capacity(tag: impl Into<Tag>, n: usize) -> TaggedDictBuilder {
        TaggedDictBuilder {
            tag: tag.into(),
            dict: IndexMap::with_capacity(n),
        }
    }

    /// Insert an untagged key/value pair into the dictionary, to later be tagged when built
    pub fn insert_untagged(&mut self, key: impl Into<String>, value: impl Into<UntaggedValue>) {
        self.dict
            .insert(key.into(), value.into().into_value(&self.tag));
    }

    ///  Insert a key/value pair into the dictionary
    pub fn insert_value(&mut self, key: impl Into<String>, value: impl Into<Value>) {
        self.dict.insert(key.into(), value.into());
    }

    /// Convert the dictionary into a tagged Value using the original tag
    pub fn into_value(self) -> Value {
        let tag = self.tag.clone();
        self.into_untagged_value().into_value(tag)
    }

    /// Convert the dictionary into an UntaggedValue
    pub fn into_untagged_value(self) -> UntaggedValue {
        UntaggedValue::Row(Dictionary { entries: self.dict })
    }

    /// Returns true if the dictionary is empty, false otherwise
    pub fn is_empty(&self) -> bool {
        self.dict.is_empty()
    }

    /// Checks if given key exists
    pub fn contains_key(&self, key: &str) -> bool {
        self.dict.contains_key(key)
    }
}

impl From<TaggedDictBuilder> for Value {
    /// Convert a builder into a tagged Value
    fn from(input: TaggedDictBuilder) -> Value {
        input.into_value()
    }
}
