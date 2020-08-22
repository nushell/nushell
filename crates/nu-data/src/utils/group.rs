use indexmap::IndexMap;
use nu_errors::ShellError;
use nu_protocol::{TaggedDictBuilder, UntaggedValue, Value};
use nu_source::Tag;
use nu_value_ext::as_string;

#[allow(clippy::type_complexity)]
pub fn group(
    values: &Value,
    grouper: &Option<Box<dyn Fn(usize, &Value) -> Result<String, ShellError> + Send>>,
    tag: impl Into<Tag>,
) -> Result<Value, ShellError> {
    let tag = tag.into();

    let mut groups: IndexMap<String, Vec<Value>> = IndexMap::new();

    for (idx, value) in values.table_entries().enumerate() {
        let group_key = if let Some(ref grouper) = grouper {
            grouper(idx, &value)
        } else {
            as_string(&value)
        };

        let group = groups.entry(group_key?).or_insert(vec![]);
        group.push((*value).clone());
    }

    let mut out = TaggedDictBuilder::new(&tag);

    for (k, v) in groups.iter() {
        out.insert_untagged(k, UntaggedValue::table(v));
    }

    Ok(out.into_value())
}
