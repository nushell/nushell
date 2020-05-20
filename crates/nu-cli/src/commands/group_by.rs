use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use indexmap::indexmap;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;

pub struct GroupBy;

#[derive(Deserialize)]
pub struct GroupByArgs {
    column_name: Option<Tagged<String>>,
}

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

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        group_by(args, registry)
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

pub fn group_by(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let name = args.call_info.name_tag.clone();
    let stream = async_stream! {
        let (GroupByArgs { column_name }, mut input) = args.process(&registry).await?;
        let values: Vec<Value> = input.collect().await;

        if values.is_empty() {
            yield Err(ShellError::labeled_error(
                    "Expected table from pipeline",
                    "requires a table input",
                    name
                ))
        } else {

            match crate::utils::data::group(column_name, &values, None, &name) {
                Ok(grouped) => yield ReturnSuccess::value(grouped),
                Err(err) => yield Err(err),
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
    crate::utils::data::group(Some(column_name.clone()), &values, None, tag)
}

#[cfg(test)]
mod tests {
    use crate::commands::group_by::group;
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
    fn groups_table_by_date_column() -> Result<(), ShellError> {
        let for_key = String::from("date").tagged_unknown();

        assert_eq!(
            group(&for_key, nu_releases_commiters(), Tag::unknown())?,
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
        let for_key = String::from("country").tagged_unknown();

        assert_eq!(
            group(&for_key, nu_releases_commiters(), Tag::unknown())?,
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
