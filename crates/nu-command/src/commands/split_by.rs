use crate::prelude::*;
use crate::utils::suggestions::suggestions;
use nu_engine::WholeStreamCommand;
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

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        split_by(args).await
    }
}

pub async fn split_by(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let name = args.call_info.name_tag.clone();
    let (SplitByArgs { column_name }, input) = args.process().await?;
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

            nu_data::utils::split(&values, &Some(block), &name)
        }
        Grouper::ByColumn(None) => {
            let block = Box::new(move |_, row: &Value| as_string(row));

            nu_data::utils::split(&values, &Some(block), &name)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::split;
    use super::ShellError;
    use nu_data::utils::helpers::committers_grouped_by_date;
    use nu_protocol::UntaggedValue;
    use nu_source::*;
    use nu_test_support::value::{date, int, row, string, table};

    #[test]
    fn splits_inner_tables_by_key() {
        let for_key = Some(String::from("country").tagged_unknown());

        assert_eq!(
            split(&for_key, &committers_grouped_by_date(), Tag::unknown()).unwrap(),
            UntaggedValue::row(indexmap! {
                "EC".into() => row(indexmap! {
                    "2019-07-23".into() => table(&[
                        row(indexmap!{"name".into() => string("AR"), "country".into() => string("EC"), "date".into() => date("2019-07-23"), "chickens".into() => int(10)})
                    ]),
                    "2019-09-24".into() => table(&[
                        row(indexmap!{"name".into() => string("AR"), "country".into() => string("EC"), "date".into() => date("2019-09-24"), "chickens".into() => int(20)})
                    ]),
                    "2019-10-10".into() => table(&[
                        row(indexmap!{"name".into() => string("AR"), "country".into() => string("EC"), "date".into() => date("2019-10-10"), "chickens".into() => int(30)})
                    ])
                }),
                "NZ".into() => row(indexmap! {
                    "2019-07-23".into() => table(&[
                        row(indexmap!{"name".into() => string("JT"), "country".into() => string("NZ"), "date".into() => date("2019-07-23"), "chickens".into() =>  int(5)})
                    ]),
                    "2019-09-24".into() => table(&[
                        row(indexmap!{"name".into() => string("JT"), "country".into() => string("NZ"), "date".into() => date("2019-09-24"), "chickens".into() => int(10)})
                    ]),
                    "2019-10-10".into() => table(&[
                        row(indexmap!{"name".into() => string("JT"), "country".into() => string("NZ"), "date".into() => date("2019-10-10"), "chickens".into() => int(15)})
                    ])
                }),
                "US".into() => row(indexmap! {
                    "2019-07-23".into() => table(&[
                        row(indexmap!{"name".into() => string("YK"), "country".into() => string("US"), "date".into() => date("2019-07-23"), "chickens".into() =>  int(2)})
                    ]),
                    "2019-09-24".into() => table(&[
                        row(indexmap!{"name".into() => string("YK"), "country".into() => string("US"), "date".into() => date("2019-09-24"), "chickens".into() =>  int(4)})
                    ]),
                    "2019-10-10".into() => table(&[
                        row(indexmap!{"name".into() => string("YK"), "country".into() => string("US"), "date".into() => date("2019-10-10"), "chickens".into() =>  int(6)})
                    ])
                })
            }).into_untagged_value()
        );
    }

    #[test]
    fn errors_if_key_within_some_inner_table_is_missing() {
        let for_key = Some(String::from("country").tagged_unknown());

        let nu_releases = row(indexmap! {
            "2019-07-23".into() =>  table(&[
                    row(indexmap!{"name".into() => string("AR"), "country".into() => string("EC"), "date".into() => string("2019-07-23")})
            ]),
            "2019-09-24".into() =>  table(&[
                    row(indexmap!{"name".into() => UntaggedValue::string("JT").into_value(Tag::from(Span::new(5,10))), "date".into() => string("2019-09-24")})
            ]),
            "October 10-2019".into() =>  table(&[
                    row(indexmap!{"name".into() => string("YK"), "country".into() => string("US"), "date".into() => string("October 10-2019")})
            ])
        });

        assert!(split(&for_key, &nu_releases, Tag::from(Span::new(5, 10))).is_err());
    }

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use super::SplitBy;
        use crate::examples::test as test_examples;

        test_examples(SplitBy {})
    }
}
