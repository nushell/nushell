use crate::commands::WholeStreamCommand;
use crate::data::TaggedListBuilder;
use crate::prelude::*;
use chrono::{DateTime, NaiveDate, Utc};
use nu_errors::ShellError;
use nu_protocol::{
    Primitive, ReturnSuccess, Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue, Value,
};
use nu_source::Tagged;
use nu_value_ext::get_data_by_key;

pub struct TSortBy;

#[derive(Deserialize)]
pub struct TSortByArgs {
    #[serde(rename(deserialize = "show-columns"))]
    show_columns: bool,
    group_by: Option<Tagged<String>>,
    #[allow(unused)]
    split_by: Option<String>,
}

impl WholeStreamCommand for TSortBy {
    fn name(&self) -> &str {
        "t-sort-by"
    }

    fn signature(&self) -> Signature {
        Signature::build("t-sort-by")
            .switch("show-columns", "Displays the column names sorted")
            .named(
                "group_by",
                SyntaxShape::String,
                "the name of the column to group by",
            )
            .named(
                "split_by",
                SyntaxShape::String,
                "the name of the column within the grouped by table to split by",
            )
    }

    fn usage(&self) -> &str {
        "Sort by the given columns."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, t_sort_by)?.run()
    }
}

fn t_sort_by(
    TSortByArgs {
        show_columns,
        group_by,
        ..
    }: TSortByArgs,
    RunnableContext { input, name, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    Ok(OutputStream::new(async_stream! {
        let values: Vec<Value> = input.values.collect().await;

        let column_grouped_by_name = if let Some(grouped_by) = group_by {
            Some(grouped_by.item().clone())
        } else {
            None
        };

        if show_columns {
            for label in columns_sorted(column_grouped_by_name, &values[0], &name).into_iter() {
                 yield ReturnSuccess::value(UntaggedValue::string(label.item).into_value(label.tag));
            }
        } else {
            match t_sort(column_grouped_by_name, None, &values[0], name) {
                Ok(sorted) => yield ReturnSuccess::value(sorted),
                Err(err) => yield Err(err)
            }
        }
    }))
}

pub fn columns_sorted(
    _group_by_name: Option<String>,
    value: &Value,
    tag: impl Into<Tag>,
) -> Vec<Tagged<String>> {
    let origin_tag = tag.into();

    match value {
        Value {
            value: UntaggedValue::Row(rows),
            ..
        } => {
            let mut keys: Vec<Value> = rows
                .entries
                .keys()
                .map(|s| s.as_ref())
                .map(|k: &str| {
                    let date = NaiveDate::parse_from_str(k, "%B %d-%Y");

                    let date = match date {
                        Ok(parsed) => UntaggedValue::Primitive(Primitive::Date(
                            DateTime::<Utc>::from_utc(parsed.and_hms(12, 34, 56), Utc),
                        )),
                        Err(_) => UntaggedValue::string(k),
                    };

                    date.into_untagged_value()
                })
                .collect();

            keys.sort();

            let keys: Vec<String> = keys
                .into_iter()
                .map(|k| match k {
                    Value {
                        value: UntaggedValue::Primitive(Primitive::Date(d)),
                        ..
                    } => format!("{}", d.format("%B %d-%Y")),
                    _ => k.as_string().unwrap(),
                })
                .collect();

            keys.into_iter().map(|k| k.tagged(&origin_tag)).collect()
        }
        _ => vec!["default".to_owned().tagged(&origin_tag)],
    }
}

pub fn t_sort(
    group_by_name: Option<String>,
    split_by_name: Option<String>,
    value: &Value,
    tag: impl Into<Tag>,
) -> Result<Value, ShellError> {
    let origin_tag = tag.into();

    match group_by_name {
        Some(column_name) => {
            let sorted_labels: Vec<Tagged<String>> =
                columns_sorted(Some(column_name), value, &origin_tag);

            match split_by_name {
                None => {
                    let mut dataset = TaggedDictBuilder::new(&origin_tag);
                    dataset.insert_value("default", value.clone());
                    let dataset = dataset.into_value();

                    let split_labels: Vec<Tagged<String>> = match &dataset {
                        Value {
                            value: UntaggedValue::Row(rows),
                            ..
                        } => {
                            let mut keys: Vec<Tagged<String>> = rows
                                .entries
                                .keys()
                                .map(|k| k.clone().tagged_unknown())
                                .collect();

                            keys.sort();

                            keys
                        }
                        _ => vec![],
                    };

                    let results: Vec<Vec<Value>> = split_labels
                        .iter()
                        .map(|split| {
                            let groups = get_data_by_key(&dataset, split.borrow_spanned());

                            sorted_labels
                                .clone()
                                .into_iter()
                                .map(|label| match &groups {
                                    Some(Value {
                                        value: UntaggedValue::Row(dict),
                                        ..
                                    }) => dict.get_data_by_key(label.borrow_spanned()).unwrap(),
                                    _ => UntaggedValue::Table(vec![]).into_value(&origin_tag),
                                })
                                .collect()
                        })
                        .collect();

                    let mut outer = TaggedListBuilder::new(&origin_tag);

                    for i in results {
                        outer.push_value(UntaggedValue::Table(i).into_value(&origin_tag));
                    }

                    Ok(UntaggedValue::Table(outer.list).into_value(&origin_tag))
                }
                Some(_) => Ok(UntaggedValue::nothing().into_value(&origin_tag)),
            }
        }
        None => Ok(UntaggedValue::nothing().into_value(&origin_tag)),
    }
}
#[cfg(test)]
mod tests {

    use crate::commands::group_by::group;
    use crate::commands::t_sort_by::{columns_sorted, t_sort};
    use crate::data::value;
    use indexmap::IndexMap;
    use nu_protocol::{UntaggedValue, Value};
    use nu_source::*;

    fn string(input: impl Into<String>) -> Value {
        UntaggedValue::string(input.into()).into_untagged_value()
    }

    fn row(entries: IndexMap<String, Value>) -> Value {
        UntaggedValue::row(entries).into_untagged_value()
    }

    fn table(list: &[Value]) -> Value {
        UntaggedValue::table(list).into_untagged_value()
    }

    fn nu_releases_grouped_by_date() -> Value {
        let key = String::from("date").tagged_unknown();
        group(&key, nu_releases_commiters(), Tag::unknown()).unwrap()
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
                indexmap! {"name".into() => string("AR"), "country".into() => string("EC"), "date".into() => string("September 24-2019")},
            ),
            row(
                indexmap! {"name".into() => string("JT"), "country".into() => string("NZ"), "date".into() => string("October 10-2019")},
            ),
            row(
                indexmap! {"name".into() => string("YK"), "country".into() => string("US"), "date".into() => string("September 24-2019")},
            ),
            row(
                indexmap! {"name".into() => string("AR"), "country".into() => string("EC"), "date".into() => string("October 10-2019")},
            ),
            row(
                indexmap! {"name".into() => string("JT"), "country".into() => string("NZ"), "date".into() => string("September 24-2019")},
            ),
            row(
                indexmap! {"name".into() => string("YK"), "country".into() => string("US"), "date".into() => string("August 23-2019")},
            ),
        ]
    }

    #[test]
    fn show_columns_sorted_given_a_column_to_sort_by() {
        let by_column = String::from("date");

        assert_eq!(
            columns_sorted(
                Some(by_column),
                &nu_releases_grouped_by_date(),
                Tag::unknown()
            ),
            vec![
                "August 23-2019".to_string().tagged_unknown(),
                "September 24-2019".to_string().tagged_unknown(),
                "October 10-2019".to_string().tagged_unknown()
            ]
        )
    }

    #[test]
    fn sorts_the_tables() {
        let group_by = String::from("date");

        assert_eq!(
            t_sort(
                Some(group_by),
                None,
                &nu_releases_grouped_by_date(),
                Tag::unknown()
            )
            .unwrap(),
            table(&[table(&[
                table(&[
                    row(
                        indexmap! {"name".into() => string("AR"), "country".into() => string("EC"), "date".into() => string("August 23-2019")}
                    ),
                    row(
                        indexmap! {"name".into() => string("JT"), "country".into() => string("NZ"), "date".into() => string("August 23-2019")}
                    ),
                    row(
                        indexmap! {"name".into() => string("YK"), "country".into() => string("US"), "date".into() => string("August 23-2019")}
                    )
                ]),
                table(&[
                    row(
                        indexmap! {"name".into() => string("AR"), "country".into() => string("EC"), "date".into() => string("September 24-2019")}
                    ),
                    row(
                        indexmap! {"name".into() => string("YK"), "country".into() => string("US"), "date".into() => string("September 24-2019")}
                    ),
                    row(
                        indexmap! {"name".into() => string("JT"), "country".into() => string("NZ"), "date".into() => string("September 24-2019")}
                    )
                ]),
                table(&[
                    row(
                        indexmap! {"name".into() => string("YK"), "country".into() => string("US"), "date".into() => string("October 10-2019")}
                    ),
                    row(
                        indexmap! {"name".into() => string("JT"), "country".into() => string("NZ"), "date".into() => string("October 10-2019")}
                    ),
                    row(
                        indexmap! {"name".into() => string("AR"), "country".into() => string("EC"), "date".into() => string("October 10-2019")}
                    )
                ]),
            ]),])
        );
    }
}
