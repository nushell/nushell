use crate::commands::WholeStreamCommand;
use crate::data::base::property_get::get_data_by_key;
use crate::data::{value, TaggedDictBuilder};
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, Value};
use nu_source::Tagged;

pub struct GroupBy;

#[derive(Deserialize)]
pub struct GroupByArgs {
    column_name: Tagged<String>,
}

impl WholeStreamCommand for GroupBy {
    fn name(&self) -> &str {
        "group-by"
    }

    fn signature(&self) -> Signature {
        Signature::build("group-by").required(
            "column_name",
            SyntaxShape::String,
            "the name of the column to group by",
        )
    }

    fn usage(&self) -> &str {
        "Creates a new table with the data from the table rows grouped by the column given."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, group_by)?.run()
    }
}

pub fn group_by(
    GroupByArgs { column_name }: GroupByArgs,
    RunnableContext { input, name, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let stream = async_stream! {
        let values: Vec<Value> = input.values.collect().await;

        if values.is_empty() {
            yield Err(ShellError::labeled_error(
                    "Expected table from pipeline",
                    "requires a table input",
                    column_name.span()
                ))
        } else {
            match group(&column_name, values, name) {
                Ok(grouped) => yield ReturnSuccess::value(grouped),
                Err(err) => yield Err(err)
            }
        }
    };

    Ok(stream.to_output_stream())
}

pub fn group(
    column_name: &Tagged<String>,
    values: Vec<Value>,
    tag: impl Into<Tag>,
) -> Result<Value, ShellError> {
    let tag = tag.into();

    let mut groups: indexmap::IndexMap<String, Vec<Value>> = indexmap::IndexMap::new();

    for value in values {
        let group_key = get_data_by_key(&value, column_name.borrow_spanned());

        if group_key.is_none() {
            let possibilities = value.data_descriptors();

            let mut possible_matches: Vec<_> = possibilities
                .iter()
                .map(|x| (natural::distance::levenshtein_distance(x, column_name), x))
                .collect();

            possible_matches.sort();

            if possible_matches.len() > 0 {
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

        let group_key = group_key.unwrap().as_string()?.to_string();
        let group = groups.entry(group_key).or_insert(vec![]);
        group.push(value);
    }

    let mut out = TaggedDictBuilder::new(&tag);

    for (k, v) in groups.iter() {
        out.insert_untagged(k, value::table(v));
    }

    Ok(out.into_value())
}

#[cfg(test)]
mod tests {
    use crate::commands::group_by::group;
    use crate::data::value;
    use indexmap::IndexMap;
    use nu_protocol::Value;
    use nu_source::*;

    fn string(input: impl Into<String>) -> Value {
        value::string(input.into()).into_untagged_value()
    }

    fn row(entries: IndexMap<String, Value>) -> Value {
        value::row(entries).into_untagged_value()
    }

    fn table(list: &Vec<Value>) -> Value {
        value::table(list).into_untagged_value()
    }

    fn nu_releases_commiters() -> Vec<Value> {
        vec![
            row(
                indexmap! {"name".into() => string("AR"), "country".into() => string("EC"), "date".into() => string("August 23-2019")},
            ),
            row(
                indexmap! {"name".into() => string("JT"), "country".into() => string("NZ"), "date".into() => string("August 23-2019")},
            ),
            row(
                indexmap! {"name".into() => string("YK"), "country".into() => string("US"), "date".into() => string("October 10-2019")},
            ),
            row(
                indexmap! {"name".into() => string("AR"), "country".into() => string("EC"), "date".into() => string("Sept 24-2019")},
            ),
            row(
                indexmap! {"name".into() => string("JT"), "country".into() => string("NZ"), "date".into() => string("October 10-2019")},
            ),
            row(
                indexmap! {"name".into() => string("YK"), "country".into() => string("US"), "date".into() => string("Sept 24-2019")},
            ),
            row(
                indexmap! {"name".into() => string("AR"), "country".into() => string("EC"), "date".into() => string("October 10-2019")},
            ),
            row(
                indexmap! {"name".into() => string("JT"), "country".into() => string("NZ"), "date".into() => string("Sept 24-2019")},
            ),
            row(
                indexmap! {"name".into() => string("YK"), "country".into() => string("US"), "date".into() => string("August 23-2019")},
            ),
        ]
    }

    #[test]
    fn groups_table_by_date_column() {
        let for_key = String::from("date").tagged_unknown();

        assert_eq!(
            group(&for_key, nu_releases_commiters(), Tag::unknown()).unwrap(),
            row(indexmap! {
                "August 23-2019".into() =>  table(&vec![
                    row(indexmap!{"name".into() => string("AR"), "country".into() => string("EC"), "date".into() => string("August 23-2019")}),
                    row(indexmap!{"name".into() => string("JT"), "country".into() => string("NZ"), "date".into() => string("August 23-2019")}),
                    row(indexmap!{"name".into() => string("YK"), "country".into() => string("US"), "date".into() => string("August 23-2019")})
                ]),
                "October 10-2019".into() =>  table(&vec![
                    row(indexmap!{"name".into() => string("YK"), "country".into() => string("US"), "date".into() => string("October 10-2019")}),
                    row(indexmap!{"name".into() => string("JT"), "country".into() => string("NZ"), "date".into() => string("October 10-2019")}),
                    row(indexmap!{"name".into() => string("AR"), "country".into() => string("EC"), "date".into() => string("October 10-2019")})
                ]),
                "Sept 24-2019".into() =>  table(&vec![
                    row(indexmap!{"name".into() => string("AR"), "country".into() => string("EC"), "date".into() => string("Sept 24-2019")}),
                    row(indexmap!{"name".into() => string("YK"), "country".into() => string("US"), "date".into() => string("Sept 24-2019")}),
                    row(indexmap!{"name".into() => string("JT"), "country".into() => string("NZ"), "date".into() => string("Sept 24-2019")})
                ]),
            })
        );
    }

    #[test]
    fn groups_table_by_country_column() {
        let for_key = String::from("country").tagged_unknown();

        assert_eq!(
            group(&for_key, nu_releases_commiters(), Tag::unknown()).unwrap(),
            row(indexmap! {
                "EC".into() =>  table(&vec![
                    row(indexmap!{"name".into() => string("AR"), "country".into() => string("EC"), "date".into() => string("August 23-2019")}),
                    row(indexmap!{"name".into() => string("AR"), "country".into() => string("EC"), "date".into() => string("Sept 24-2019")}),
                    row(indexmap!{"name".into() => string("AR"), "country".into() => string("EC"), "date".into() => string("October 10-2019")})
                ]),
                "NZ".into() =>  table(&vec![
                    row(indexmap!{"name".into() => string("JT"), "country".into() => string("NZ"), "date".into() => string("August 23-2019")}),
                    row(indexmap!{"name".into() => string("JT"), "country".into() => string("NZ"), "date".into() => string("October 10-2019")}),
                    row(indexmap!{"name".into() => string("JT"), "country".into() => string("NZ"), "date".into() => string("Sept 24-2019")})
                ]),
                "US".into() =>  table(&vec![
                    row(indexmap!{"name".into() => string("YK"), "country".into() => string("US"), "date".into() => string("October 10-2019")}),
                    row(indexmap!{"name".into() => string("YK"), "country".into() => string("US"), "date".into() => string("Sept 24-2019")}),
                    row(indexmap!{"name".into() => string("YK"), "country".into() => string("US"), "date".into() => string("August 23-2019")}),
                ]),
            })
        );
    }
}
