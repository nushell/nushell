use nu_errors::ShellError;
use nu_protocol::{SpannedTypeName, TaggedDictBuilder, UntaggedValue, Value};
use nu_source::Tag;

use crate::utils::data::group;

#[allow(clippy::type_complexity)]
pub fn split(
    value: &Value,
    splitter: &Option<Box<dyn Fn(usize, &Value) -> Result<String, ShellError> + Send>>,
    tag: impl Into<Tag>,
) -> Result<Value, ShellError> {
    let tag = tag.into();

    let mut splits = indexmap::IndexMap::new();
    let mut out = TaggedDictBuilder::new(&tag);

    if splitter.is_none() {
        out.insert_untagged("table", UntaggedValue::table(&[value.clone()]));
        return Ok(out.into_value());
    }

    for (column, value) in value.row_entries() {
        if !&value.is_table() {
            return Err(ShellError::type_error(
                "a table value",
                value.spanned_type_name(),
            ));
        }

        match group(&value, splitter, &tag) {
            Ok(grouped) => {
                for (split_label, subset) in grouped.row_entries() {
                    let s = splits
                        .entry(split_label.clone())
                        .or_insert(indexmap::IndexMap::new());

                    if !&subset.is_table() {
                        return Err(ShellError::type_error(
                            "a table value",
                            subset.spanned_type_name(),
                        ));
                    }

                    s.insert(column.clone(), subset.clone());
                }
            }
            Err(err) => return Err(err),
        }
    }

    let mut out = TaggedDictBuilder::new(&tag);

    for (k, v) in splits.into_iter() {
        out.insert_untagged(k, UntaggedValue::row(v));
    }

    Ok(out.into_value())
}
