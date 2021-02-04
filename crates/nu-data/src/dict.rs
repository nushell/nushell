use derive_new::new;
use nu_protocol::{Dictionary, MaybeOwned, Primitive, UntaggedValue, Value};
use nu_source::{DbgDocBldr, DebugDocBuilder, PrettyDebug, Spanned, Tag};

#[derive(Debug, new)]
struct DebugEntry<'a> {
    key: &'a str,
    value: &'a Value,
}

impl<'a> PrettyDebug for DebugEntry<'a> {
    fn pretty(&self) -> DebugDocBuilder {
        (DbgDocBldr::key(self.key.to_string())
            + DbgDocBldr::equals()
            + self.value.pretty().into_value())
        .group()
    }
}

pub trait DictionaryExt {
    fn get_data(&self, desc: &str) -> MaybeOwned<'_, Value>;

    fn keys(&self) -> indexmap::map::Keys<String, Value>;
    fn get_data_by_key(&self, name: Spanned<&str>) -> Option<Value>;
    fn get_mut_data_by_key(&mut self, name: &str) -> Option<&mut Value>;
    fn insert_data_at_key(&mut self, name: &str, value: Value);
}

impl DictionaryExt for Dictionary {
    fn get_data(&self, desc: &str) -> MaybeOwned<'_, Value> {
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
        self.entries
            .iter_mut()
            .find(|(desc_name, _)| *desc_name == name)
            .map_or_else(|| None, |x| Some(x.1))
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
