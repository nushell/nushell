use crate::commands::WholeStreamCommand;
use crate::data::{TaggedDictBuilder, TaggedListBuilder};
use crate::errors::ShellError;
use crate::prelude::*;
use chrono::{DateTime, NaiveDate, Utc};

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
        let values: Vec<Tagged<Value>> = input.values.collect().await;

        let column_grouped_by_name = if let Some(grouped_by) = group_by {
            Some(grouped_by.item().clone())
        } else {
            None
        };

        if show_columns {
            for label in columns_sorted(column_grouped_by_name, &values[0], &name).into_iter() {
                 yield ReturnSuccess::value(Value::string(label.item).tagged(label.tag));
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
    value: &Tagged<Value>,
    tag: impl Into<Tag>,
) -> Vec<Tagged<String>> {
    let origin_tag = tag.into();

    match value {
        Tagged {
            item: Value::Row(rows),
            ..
        } => {
            let mut keys: Vec<Tagged<Value>> =
                rows.entries
                    .keys()
                    .map(|s| s.as_ref())
                    .map(|k: &str| {
                        let date = NaiveDate::parse_from_str(k, "%B %d-%Y");

                        let date = match date {
                            Ok(parsed) => Value::Primitive(Primitive::Date(
                                DateTime::<Utc>::from_utc(parsed.and_hms(12, 34, 56), Utc),
                            )),
                            Err(_) => Value::string(k),
                        };

                        date.tagged_unknown()
                    })
                    .collect();

            keys.sort();

            let keys: Vec<String> = keys
                .into_iter()
                .map(|k| match k {
                    Tagged {
                        item: Value::Primitive(Primitive::Date(d)),
                        ..
                    } => format!("{}", d.format("%B %d-%Y")),
                    _ => k.as_string().unwrap(),
                })
                .collect();

            keys.into_iter().map(|k| k.tagged(&origin_tag)).collect()
        }
        _ => vec![format!("default").tagged(&origin_tag)],
    }
}

pub fn t_sort(
    group_by_name: Option<String>,
    split_by_name: Option<String>,
    value: &Tagged<Value>,
    tag: impl Into<Tag>,
) -> Result<Tagged<Value>, ShellError> {
    let origin_tag = tag.into();

    match group_by_name {
        Some(column_name) => {
            let sorted_labels: Vec<Tagged<String>> =
                columns_sorted(Some(column_name), value, &origin_tag);

            match split_by_name {
                None => {
                    let mut dataset = TaggedDictBuilder::new(&origin_tag);
                    dataset.insert_tagged("default", value.clone());
                    let dataset = dataset.into_tagged_value();

                    let split_labels: Vec<Tagged<String>> = match &dataset {
                        Tagged {
                            item: Value::Row(rows),
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

                    let results: Vec<Vec<Tagged<Value>>> = split_labels
                        .iter()
                        .map(|split| {
                            let groups = dataset.get_data_by_key(split.borrow_spanned());

                            sorted_labels
                                .clone()
                                .into_iter()
                                .map(|label| match &groups {
                                    Some(Tagged {
                                        item: Value::Row(dict),
                                        ..
                                    }) => dict
                                        .get_data_by_key(label.borrow_spanned())
                                        .unwrap()
                                        .clone(),
                                    _ => Value::Table(vec![]).tagged(&origin_tag),
                                })
                                .collect()
                        })
                        .collect();

                    let mut outer = TaggedListBuilder::new(&origin_tag);

                    for i in results {
                        outer.insert_tagged(Value::Table(i).tagged(&origin_tag));
                    }

                    return Ok(Value::Table(outer.list).tagged(&origin_tag));
                }
                Some(_) => return Ok(Value::nothing().tagged(&origin_tag)),
            }
        }
        None => return Ok(Value::nothing().tagged(&origin_tag)),
    }
}
#[cfg(test)]
mod tests {

    use crate::commands::group_by::group;
    use crate::commands::t_sort_by::{columns_sorted, t_sort};
    use crate::data::meta::*;
    use crate::Value;
    use indexmap::IndexMap;

    fn string(input: impl Into<String>) -> Tagged<Value> {
        Value::string(input.into()).tagged_unknown()
    }

    fn row(entries: IndexMap<String, Tagged<Value>>) -> Tagged<Value> {
        Value::row(entries).tagged_unknown()
    }

    fn table(list: &Vec<Tagged<Value>>) -> Tagged<Value> {
        Value::table(list).tagged_unknown()
    }

    fn nu_releases_grouped_by_date() -> Tagged<Value> {
        let key = String::from("date").tagged_unknown();
        group(&key, nu_releases_commiters(), Tag::unknown()).unwrap()
    }

    fn nu_releases_commiters() -> Vec<Tagged<Value>> {
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
                format!("August 23-2019").tagged_unknown(),
                format!("September 24-2019").tagged_unknown(),
                format!("October 10-2019").tagged_unknown()
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
            table(&vec![table(&vec![
                table(&vec![
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
                table(&vec![
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
                table(&vec![
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
