use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use indexmap::indexmap;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;
use nu_value_ext::as_string;

pub struct GroupBy;

#[derive(Deserialize)]
pub struct GroupByArgs {
    column_name: Option<Tagged<String>>,
}

#[async_trait]
impl WholeStreamCommand for GroupBy {
    fn name(&self) -> &str {
        "group-by"
    }

    fn signature(&self) -> Signature {
        Signature::build("group-by").optional(
            "column_name",
            SyntaxShape::String,
            "the name of the column to group by",
        )
    }

    fn usage(&self) -> &str {
        "Creates a new table with the data from the table rows grouped by the column given."
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        group_by(args, registry).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Group items by type",
                example: r#"ls | group-by type"#,
                result: None,
            },
            Example {
                description: "Group items by their value",
                example: "echo [1 3 1 3 2 1 1] | group-by",
                result: Some(vec![UntaggedValue::row(indexmap! {
                    "1".to_string() => UntaggedValue::Table(vec![
                        UntaggedValue::int(1).into(),
                        UntaggedValue::int(1).into(),
                        UntaggedValue::int(1).into(),
                        UntaggedValue::int(1).into(),
                    ]).into(),

                    "3".to_string() => UntaggedValue::Table(vec![
                        UntaggedValue::int(3).into(),
                        UntaggedValue::int(3).into(),
                    ]).into(),

                    "2".to_string() => UntaggedValue::Table(vec![
                        UntaggedValue::int(2).into(),
                    ]).into(),
                })
                .into()]),
            },
        ]
    }
}

enum Grouper {
    ByColumn(Option<Tagged<String>>),
}

pub async fn group_by(
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let name = args.call_info.name_tag.clone();
    let (GroupByArgs { column_name }, input) = args.process(&registry).await?;
    let values: Vec<Value> = input.collect().await;

    if values.is_empty() {
        return Err(ShellError::labeled_error(
            "Expected table from pipeline",
            "requires a table input",
            name,
        ));
    }

    let values = UntaggedValue::table(&values).into_value(&name);

    match group(&column_name, &values, name) {
        Ok(grouped) => Ok(OutputStream::one(ReturnSuccess::value(grouped))),
        Err(reason) => Err(reason),
    }
}

pub fn suggestions(tried: Tagged<&str>, for_value: &Value) -> ShellError {
    let possibilities = for_value.data_descriptors();

    let mut possible_matches: Vec<_> = possibilities
        .iter()
        .map(|x| (natural::distance::levenshtein_distance(x, &tried), x))
        .collect();

    possible_matches.sort();

    if !possible_matches.is_empty() {
        ShellError::labeled_error(
            "Unknown column",
            format!("did you mean '{}'?", possible_matches[0].1),
            tried.tag(),
        )
    } else {
        ShellError::labeled_error(
            "Unknown column",
            "row does not contain this column",
            tried.tag(),
        )
    }
}

pub fn group(
    column_name: &Option<Tagged<String>>,
    values: &Value,
    tag: impl Into<Tag>,
) -> Result<Value, ShellError> {
    let name = tag.into();

    let grouper = if let Some(column_name) = column_name {
        Grouper::ByColumn(Some(column_name.clone()))
    } else {
        Grouper::ByColumn(None)
    };

    match grouper {
        Grouper::ByColumn(Some(column_name)) => {
            let block = Box::new(move |row: &Value| {
                match row.get_data_by_key(column_name.borrow_spanned()) {
                    Some(group_key) => Ok(as_string(&group_key)?),
                    None => Err(suggestions(column_name.borrow_tagged(), &row)),
                }
            });

            crate::utils::data::group(&values, &Some(block), &name)
        }
        Grouper::ByColumn(None) => {
            let block = Box::new(move |row: &Value| match as_string(row) {
                Ok(group_key) => Ok(group_key),
                Err(reason) => Err(reason),
            });

            crate::utils::data::group(&values, &Some(block), &name)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::group;
    use indexmap::IndexMap;
    use nu_errors::ShellError;
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

    fn nu_releases_committers() -> Vec<Value> {
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
    fn groups_table_by_date_column() -> Result<(), ShellError> {
        let for_key = Some(String::from("date").tagged_unknown());
        let sample = table(&nu_releases_committers());

        assert_eq!(
            group(&for_key, &sample, Tag::unknown())?,
            row(indexmap! {
                "August 23-2019".into() =>  table(&[
                    row(indexmap!{"name".into() => string("AR"), "country".into() => string("EC"), "date".into() => string("August 23-2019")}),
                    row(indexmap!{"name".into() => string("JT"), "country".into() => string("NZ"), "date".into() => string("August 23-2019")}),
                    row(indexmap!{"name".into() => string("YK"), "country".into() => string("US"), "date".into() => string("August 23-2019")})
                ]),
                "October 10-2019".into() =>  table(&[
                    row(indexmap!{"name".into() => string("YK"), "country".into() => string("US"), "date".into() => string("October 10-2019")}),
                    row(indexmap!{"name".into() => string("JT"), "country".into() => string("NZ"), "date".into() => string("October 10-2019")}),
                    row(indexmap!{"name".into() => string("AR"), "country".into() => string("EC"), "date".into() => string("October 10-2019")})
                ]),
                "Sept 24-2019".into() =>  table(&[
                    row(indexmap!{"name".into() => string("AR"), "country".into() => string("EC"), "date".into() => string("Sept 24-2019")}),
                    row(indexmap!{"name".into() => string("YK"), "country".into() => string("US"), "date".into() => string("Sept 24-2019")}),
                    row(indexmap!{"name".into() => string("JT"), "country".into() => string("NZ"), "date".into() => string("Sept 24-2019")})
                ]),
            })
        );

        Ok(())
    }

    #[test]
    fn groups_table_by_country_column() -> Result<(), ShellError> {
        let for_key = Some(String::from("country").tagged_unknown());
        let sample = table(&nu_releases_committers());

        assert_eq!(
            group(&for_key, &sample, Tag::unknown())?,
            row(indexmap! {
                "EC".into() =>  table(&[
                    row(indexmap!{"name".into() => string("AR"), "country".into() => string("EC"), "date".into() => string("August 23-2019")}),
                    row(indexmap!{"name".into() => string("AR"), "country".into() => string("EC"), "date".into() => string("Sept 24-2019")}),
                    row(indexmap!{"name".into() => string("AR"), "country".into() => string("EC"), "date".into() => string("October 10-2019")})
                ]),
                "NZ".into() =>  table(&[
                    row(indexmap!{"name".into() => string("JT"), "country".into() => string("NZ"), "date".into() => string("August 23-2019")}),
                    row(indexmap!{"name".into() => string("JT"), "country".into() => string("NZ"), "date".into() => string("October 10-2019")}),
                    row(indexmap!{"name".into() => string("JT"), "country".into() => string("NZ"), "date".into() => string("Sept 24-2019")})
                ]),
                "US".into() =>  table(&[
                    row(indexmap!{"name".into() => string("YK"), "country".into() => string("US"), "date".into() => string("October 10-2019")}),
                    row(indexmap!{"name".into() => string("YK"), "country".into() => string("US"), "date".into() => string("Sept 24-2019")}),
                    row(indexmap!{"name".into() => string("YK"), "country".into() => string("US"), "date".into() => string("August 23-2019")}),
                ]),
            })
        );

        Ok(())
    }

    #[test]
    fn examples_work_as_expected() {
        use super::GroupBy;
        use crate::examples::test as test_examples;

        test_examples(GroupBy {})
    }
}
