use indexmap::IndexMap;
use nu_errors::ShellError;
use nu_protocol::{TaggedDictBuilder, UntaggedValue, Value};
use nu_source::{Tag, Tagged};
use nu_value_ext::{as_string, get_data_by_key};

#[allow(clippy::type_complexity)]
pub fn group(
    column_name: Tagged<String>,
    values: &[Value],
    grouper: Option<Box<dyn Fn(&Value) -> Result<String, ShellError> + Send>>,
    tag: impl Into<Tag>,
) -> Result<Value, ShellError> {
    let tag = tag.into();

    let mut groups: IndexMap<String, Vec<Value>> = IndexMap::new();

    for value in values {
        let group_key = get_data_by_key(&value, column_name.borrow_spanned());

        if let Some(group_key) = group_key {
            let group_key = if let Some(ref grouper) = grouper {
                grouper(&group_key)
            } else {
                as_string(&group_key)
            };
            let group = groups.entry(group_key?).or_insert(vec![]);
            group.push((*value).clone());
        } else {
            let possibilities = value.data_descriptors();

            let mut possible_matches: Vec<_> = possibilities
                .iter()
                .map(|x| (natural::distance::levenshtein_distance(x, &column_name), x))
                .collect();

            possible_matches.sort();

            if !possible_matches.is_empty() {
                return Err(ShellError::labeled_error(
                    "Unknown column",
                    format!("did you mean '{}'?", possible_matches[0].1),
                    column_name.tag(),
                ));
            } else {
                return Err(ShellError::labeled_error(
                    "Unknown column",
                    "row does not contain this column",
                    column_name.tag(),
                ));
            }
        }
    }

    let mut out = TaggedDictBuilder::new(&tag);

    for (k, v) in groups.iter() {
        out.insert_untagged(k, UntaggedValue::table(v));
    }

    Ok(out.into_value())
}
