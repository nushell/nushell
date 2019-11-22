use crate::data::{Primitive, Value};
use crate::prelude::*;
use crate::traits::{DebugDocBuilder as b, PrettyDebug};
use derive_new::new;
use getset::Getters;
use indexmap::IndexMap;
use pretty::{BoxAllocator, DocAllocator};
use serde::{Deserialize, Serialize};
use std::cmp::{Ordering, PartialOrd};

#[derive(Debug, Default, Eq, PartialEq, Serialize, Deserialize, Clone, Getters, new)]
pub struct Dictionary {
    #[get = "pub"]
    pub entries: IndexMap<String, Tagged<Value>>,
}

#[derive(Debug, new)]
struct DebugEntry<'a> {
    key: &'a str,
    value: &'a Tagged<Value>,
}

impl<'a> PrettyDebug for DebugEntry<'a> {
    fn pretty_debug(&self) -> DebugDocBuilder {
        (b::key(self.key.to_string()) + b::equals() + self.value.item.pretty_debug().as_value())
            .group()
        // BoxAllocator
        //     .text(self.key.to_string())
        //     .annotate(ShellAnnotation::style("key"))
        //     .append(
        //         BoxAllocator
        //             .text("=")
        //             .annotate(ShellAnnotation::style("equals")),
        //     )
        //     .append({
        //         self.value
        //             .item
        //             .pretty_debug()
        //             .inner
        //             .annotate(ShellAnnotation::style("value"))
        //     })
        //     .group()
        //     .into()
    }
}

impl PrettyDebug for Dictionary {
    fn pretty_debug(&self) -> DebugDocBuilder {
        BoxAllocator
            .text("(")
            .append(
                BoxAllocator
                    .intersperse(
                        self.entries()
                            .iter()
                            .map(|(key, value)| DebugEntry::new(key, value).to_doc()),
                        BoxAllocator.space(),
                    )
                    .nest(1)
                    .group(),
            )
            .append(BoxAllocator.text(")"))
            .into()
    }
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
            Value::Row(d) => self == d,
            _ => false,
        }
    }
}

impl Dictionary {
    pub fn get_data(&self, desc: &String) -> MaybeOwned<'_, Value> {
        match self.entries.get(desc) {
            Some(v) => MaybeOwned::Borrowed(v),
            None => MaybeOwned::Owned(Value::Primitive(Primitive::Nothing)),
        }
    }

    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.entries.keys()
    }

    pub(crate) fn get_data_by_key(&self, name: Spanned<&str>) -> Option<Tagged<Value>> {
        let result = self
            .entries
            .iter()
            .find(|(desc_name, _)| *desc_name == name.item)?
            .1;

        Some(
            result
                .item
                .clone()
                .tagged(Tag::new(result.anchor(), name.span)),
        )
    }

    pub(crate) fn get_mut_data_by_key(&mut self, name: &str) -> Option<&mut Tagged<Value>> {
        match self
            .entries
            .iter_mut()
            .find(|(desc_name, _)| *desc_name == name)
        {
            Some((_, v)) => Some(v),
            None => None,
        }
    }

    pub(crate) fn insert_data_at_key(&mut self, name: &str, value: Tagged<Value>) {
        self.entries.insert(name.to_string(), value);
    }
}

#[derive(Debug)]
pub struct TaggedListBuilder {
    tag: Tag,
    pub list: Vec<Tagged<Value>>,
}

impl TaggedListBuilder {
    pub fn new(tag: impl Into<Tag>) -> TaggedListBuilder {
        TaggedListBuilder {
            tag: tag.into(),
            list: vec![],
        }
    }

    pub fn push(&mut self, value: impl Into<Value>) {
        self.list.push(value.into().tagged(&self.tag));
    }

    pub fn insert_tagged(&mut self, value: impl Into<Tagged<Value>>) {
        self.list.push(value.into());
    }

    pub fn into_tagged_value(self) -> Tagged<Value> {
        Value::Table(self.list).tagged(self.tag)
    }
}

impl From<TaggedListBuilder> for Tagged<Value> {
    fn from(input: TaggedListBuilder) -> Tagged<Value> {
        input.into_tagged_value()
    }
}

#[derive(Debug)]
pub struct TaggedDictBuilder {
    tag: Tag,
    dict: IndexMap<String, Tagged<Value>>,
}

impl TaggedDictBuilder {
    pub fn new(tag: impl Into<Tag>) -> TaggedDictBuilder {
        TaggedDictBuilder {
            tag: tag.into(),
            dict: IndexMap::default(),
        }
    }

    pub fn build(tag: impl Into<Tag>, block: impl FnOnce(&mut TaggedDictBuilder)) -> Tagged<Value> {
        let mut builder = TaggedDictBuilder::new(tag);
        block(&mut builder);
        builder.into_tagged_value()
    }

    pub fn with_capacity(tag: impl Into<Tag>, n: usize) -> TaggedDictBuilder {
        TaggedDictBuilder {
            tag: tag.into(),
            dict: IndexMap::with_capacity(n),
        }
    }

    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<Value>) {
        self.dict.insert(key.into(), value.into().tagged(&self.tag));
    }

    pub fn insert_tagged(&mut self, key: impl Into<String>, value: impl Into<Tagged<Value>>) {
        self.dict.insert(key.into(), value.into());
    }

    pub fn into_tagged_value(self) -> Tagged<Value> {
        self.into_tagged_dict().map(Value::Row)
    }

    pub fn into_tagged_dict(self) -> Tagged<Dictionary> {
        Dictionary { entries: self.dict }.tagged(self.tag)
    }

    pub fn is_empty(&self) -> bool {
        self.dict.is_empty()
    }
}

impl From<TaggedDictBuilder> for Tagged<Value> {
    fn from(input: TaggedDictBuilder) -> Tagged<Value> {
        input.into_tagged_value()
    }
}
