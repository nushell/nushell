use crate::prelude::*;
use derive_new::new;
use indexmap::IndexMap;
use nu_protocol::{Dictionary, Primitive, UntaggedValue, Value};
use nu_source::{b, PrettyDebug, Spanned};

#[derive(Debug, new)]
struct DebugEntry<'a> {
    key: &'a str,
    value: &'a Value,
}

impl<'a> PrettyDebug for DebugEntry<'a> {
    fn pretty(&self) -> DebugDocBuilder {
        (b::key(self.key.to_string()) + b::equals() + self.value.pretty().as_value()).group()
    }
}

pub trait DictionaryExt {
    fn get_data(&self, desc: &String) -> MaybeOwned<'_, Value>;

    fn keys(&self) -> indexmap::map::Keys<String, Value>;
    fn get_data_by_key(&self, name: Spanned<&str>) -> Option<Value>;
    fn get_mut_data_by_key(&mut self, name: &str) -> Option<&mut Value>;
    fn insert_data_at_key(&mut self, name: &str, value: Value);
}

impl DictionaryExt for Dictionary {
    fn get_data(&self, desc: &String) -> MaybeOwned<'_, Value> {
        match self.entries.get(desc) {
            Some(v) => MaybeOwned::Borrowed(v),
            None => MaybeOwned::Owned(
                UntaggedValue::Primitive(Primitive::Nothing).into_untagged_value(),
            ),
        }
    }

    fn keys(&self) -> indexmap::map::Keys<String, Value> {
        self.entries.keys()
    }

    fn get_data_by_key(&self, name: Spanned<&str>) -> Option<Value> {
        let result = self
            .entries
            .iter()
            .find(|(desc_name, _)| *desc_name == name.item)?
            .1;

        Some(
            result
                .value
                .clone()
                .into_value(Tag::new(result.anchor(), name.span)),
        )
    }

    fn get_mut_data_by_key(&mut self, name: &str) -> Option<&mut Value> {
        match self
            .entries
            .iter_mut()
            .find(|(desc_name, _)| *desc_name == name)
        {
            Some((_, v)) => Some(v),
            None => None,
        }
    }

    fn insert_data_at_key(&mut self, name: &str, value: Value) {
        self.entries.insert(name.to_string(), value);
    }
}

#[derive(Debug)]
pub struct TaggedListBuilder {
    tag: Tag,
    pub list: Vec<Value>,
}

impl TaggedListBuilder {
    pub fn new(tag: impl Into<Tag>) -> TaggedListBuilder {
        TaggedListBuilder {
            tag: tag.into(),
            list: vec![],
        }
    }

    pub fn push_value(&mut self, value: impl Into<Value>) {
        self.list.push(value.into());
    }

    pub fn push_untagged(&mut self, value: impl Into<UntaggedValue>) {
        self.list.push(value.into().into_value(self.tag.clone()));
    }

    pub fn into_value(self) -> Value {
        UntaggedValue::Table(self.list).into_value(self.tag)
    }

    pub fn into_untagged_value(self) -> UntaggedValue {
        UntaggedValue::Table(self.list).into_value(self.tag).value
    }
}

impl From<TaggedListBuilder> for Value {
    fn from(input: TaggedListBuilder) -> Value {
        input.into_value()
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
