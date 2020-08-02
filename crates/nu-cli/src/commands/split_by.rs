use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, Value};
use nu_source::Tagged;
use nu_value_ext::as_string;

pub struct SplitBy;

#[derive(Deserialize)]
pub struct SplitByArgs {
    column_name: Option<Tagged<String>>,
}

#[async_trait]
impl WholeStreamCommand for SplitBy {
    fn name(&self) -> &str {
        "split-by"
    }

    fn signature(&self) -> Signature {
        Signature::build("split-by").optional(
            "column_name",
            SyntaxShape::String,
            "the name of the column within the nested table to split by",
        )
    }

    fn usage(&self) -> &str {
        "Creates a new table with the data from the inner tables split by the column given."
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        split_by(args, registry).await
    }
}

pub async fn split_by(
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let name = args.call_info.name_tag.clone();
    let (SplitByArgs { column_name }, input) = args.process(&registry).await?;
    let values: Vec<Value> = input.collect().await;

    if values.len() > 1 || values.is_empty() {
        return Err(ShellError::labeled_error(
            "Expected table from pipeline",
            "requires a table input",
            name,
        ));
    }

    let split = split(&column_name, &values[0], &name)?;
    Ok(OutputStream::one(ReturnSuccess::value(split)))
}

enum Grouper {
    ByColumn(Option<Tagged<String>>),
}

pub fn split(
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
            let block = Box::new(move |_, row: &Value| {
                match row.get_data_by_key(column_name.borrow_spanned()) {
                    Some(group_key) => Ok(as_string(&group_key)?),
                    None => Err(suggestions(column_name.borrow_tagged(), &row)),
                }
            });

            crate::utils::data::split(&values, &Some(block), &name)
        }
        Grouper::ByColumn(None) => {
            let block = Box::new(move |_, row: &Value| as_string(row));

            crate::utils::data::split(&values, &Some(block), &name)
        }
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

#[cfg(test)]
mod tests {
    use super::split;
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

    fn nu_releases_grouped_by_date() -> Result<Value, ShellError> {
        let key = Some(String::from("date").tagged_unknown());
        let sample = table(&nu_releases_committers());
        group(&key, &sample, Tag::unknown())
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
    fn splits_inner_tables_by_key() -> Result<(), ShellError> {
        let for_key = Some(String::from("country").tagged_unknown());

        assert_eq!(
            split(&for_key, &nu_releases_grouped_by_date()?, Tag::unknown())?,
            UntaggedValue::row(indexmap! {
                "EC".into() => row(indexmap! {
                    "August 23-2019".into() => table(&[
                        row(indexmap!{"name".into() => string("AR"), "country".into() => string("EC"), "date".into() => string("August 23-2019")})
                    ]),
                    "Sept 24-2019".into() => table(&[
                        row(indexmap!{"name".into() => string("AR"), "country".into() => string("EC"), "date".into() => string("Sept 24-2019")})
                    ]),
                    "October 10-2019".into() => table(&[
                        row(indexmap!{"name".into() => string("AR"), "country".into() => string("EC"), "date".into() => string("October 10-2019")})
                    ])
                }),
                "NZ".into() => row(indexmap! {
                    "August 23-2019".into() => table(&[
                        row(indexmap!{"name".into() => string("JT"), "country".into() => string("NZ"), "date".into() => string("August 23-2019")})
                    ]),
                    "Sept 24-2019".into() => table(&[
                        row(indexmap!{"name".into() => string("JT"), "country".into() => string("NZ"), "date".into() => string("Sept 24-2019")})
                    ]),
                    "October 10-2019".into() => table(&[
                        row(indexmap!{"name".into() => string("JT"), "country".into() => string("NZ"), "date".into() => string("October 10-2019")})
                    ])
                }),
                "US".into() => row(indexmap! {
                    "August 23-2019".into() => table(&[
                        row(indexmap!{"name".into() => string("YK"), "country".into() => string("US"), "date".into() => string("August 23-2019")})
                    ]),
                    "Sept 24-2019".into() => table(&[
                        row(indexmap!{"name".into() => string("YK"), "country".into() => string("US"), "date".into() => string("Sept 24-2019")})
                    ]),
                    "October 10-2019".into() => table(&[
                        row(indexmap!{"name".into() => string("YK"), "country".into() => string("US"), "date".into() => string("October 10-2019")})
                    ])
                })
            }).into_untagged_value()
        );

        Ok(())
    }

    #[test]
    fn errors_if_key_within_some_inner_table_is_missing() {
        let for_key = Some(String::from("country").tagged_unknown());

        let nu_releases = row(indexmap! {
            "August 23-2019".into() =>  table(&[
                    row(indexmap!{"name".into() => string("AR"), "country".into() => string("EC"), "date".into() => string("August 23-2019")})
            ]),
            "Sept 24-2019".into() =>  table(&[
                    row(indexmap!{"name".into() => UntaggedValue::string("JT").into_value(Tag::from(Span::new(5,10))), "date".into() => string("Sept 24-2019")})
            ]),
            "October 10-2019".into() =>  table(&[
                    row(indexmap!{"name".into() => string("YK"), "country".into() => string("US"), "date".into() => string("October 10-2019")})
            ])
        });

        assert!(split(&for_key, &nu_releases, Tag::from(Span::new(5, 10))).is_err());
    }

    #[test]
    fn examples_work_as_expected() {
        use super::SplitBy;
        use crate::examples::test as test_examples;

        test_examples(SplitBy {})
    }
}
