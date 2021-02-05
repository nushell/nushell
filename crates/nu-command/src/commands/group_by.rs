use crate::prelude::*;
use crate::utils::suggestions::suggestions;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;
use nu_value_ext::as_string;

pub struct Command;

#[derive(Deserialize)]
pub struct Arguments {
    grouper: Option<Value>,
}

#[async_trait]
impl WholeStreamCommand for Command {
    fn name(&self) -> &str {
        "group-by"
    }

    fn signature(&self) -> Signature {
        Signature::build("group-by").optional(
            "grouper",
            SyntaxShape::Any,
            "the grouper value to use",
        )
    }

    fn usage(&self) -> &str {
        "Create a new table grouped."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        group_by(args).await
    }

    #[allow(clippy::unwrap_used)]
    fn examples(&self) -> Vec<Example> {
        use nu_data::value::date_naive_from_str as date;

        vec![
            Example {
                description: "group items by column named \"type\"",
                example: r#"ls | group-by type"#,
                result: Some(vec![UntaggedValue::row(indexmap! {
                    "File".to_string() => UntaggedValue::Table(vec![
                        UntaggedValue::row(indexmap! {
                                "name".to_string() =>          UntaggedValue::string("Andres.txt").into(),
                                "type".to_string() =>                UntaggedValue::string("File").into(),
                            "chickens".to_string() =>                       UntaggedValue::int(10).into(),
                            "modified".to_string() => date("2019-07-23".tagged_unknown()).unwrap().into(),
                        }).into(),
                        UntaggedValue::row(indexmap! {
                                "name".to_string() =>          UntaggedValue::string("Darren.txt").into(),
                                "type".to_string() =>                UntaggedValue::string("File").into(),
                            "chickens".to_string() =>                       UntaggedValue::int(20).into(),
                            "modified".to_string() => date("2019-09-24".tagged_unknown()).unwrap().into(),
                        }).into(),
                    ]).into(),
                    "Dir".to_string() => UntaggedValue::Table(vec![
                        UntaggedValue::row(indexmap! {
                                "name".to_string() =>            UntaggedValue::string("Jonathan").into(),
                                "type".to_string() =>                 UntaggedValue::string("Dir").into(),
                            "chickens".to_string() =>                        UntaggedValue::int(5).into(),
                            "modified".to_string() => date("2019-07-23".tagged_unknown()).unwrap().into(),
                        }).into(),
                        UntaggedValue::row(indexmap! {
                                "name".to_string() =>              UntaggedValue::string("Yehuda").into(),
                                "type".to_string() =>                 UntaggedValue::string("Dir").into(),
                            "chickens".to_string() =>                        UntaggedValue::int(4).into(),
                            "modified".to_string() => date("2019-09-24".tagged_unknown()).unwrap().into(),
                        }).into(),
                    ]).into(),
                })
                .into()]),
            },
            Example {
                description: "you can also group by raw values by leaving out the argument",
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
            Example {
                description:
                    "use the block form to generate a grouping key when each row gets processed",
                example: "echo [1 3 1 3 2 1 1] | group-by { = ($it - 1) mod 3 }",
                result: Some(vec![UntaggedValue::row(indexmap! {
                    "0".to_string() => UntaggedValue::Table(vec![
                        UntaggedValue::int(1).into(),
                        UntaggedValue::int(1).into(),
                        UntaggedValue::int(1).into(),
                        UntaggedValue::int(1).into(),

                    ]).into(),
                    "2".to_string() => UntaggedValue::Table(vec![
                        UntaggedValue::int(3).into(),
                        UntaggedValue::int(3).into(),
                    ]).into(),
                    "1".to_string() => UntaggedValue::Table(vec![
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
    ByBlock,
}

pub async fn group_by(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let name = args.call_info.name_tag.clone();
    let context = Arc::new(EvaluationContext::from_args(&args));
    let (Arguments { grouper }, input) = args.process().await?;

    let values: Vec<Value> = input.collect().await;
    let mut keys: Vec<Result<String, ShellError>> = vec![];
    let mut group_strategy = Grouper::ByColumn(None);

    match grouper {
        Some(Value {
            value: UntaggedValue::Block(block_given),
            ..
        }) => {
            let block = Arc::new(block_given);
            let error_key = "error";

            for value in values.iter() {
                let run = block.clone();
                let context = context.clone();

                match crate::commands::each::process_row(run, context, value.clone()).await {
                    Ok(mut s) => {
                        let collection: Vec<Result<ReturnSuccess, ShellError>> =
                            s.drain_vec().await;

                        if collection.len() > 1 {
                            return Err(ShellError::labeled_error(
                                "expected one value from the block",
                                "requires a table with one value for grouping",
                                &name,
                            ));
                        }

                        let value = match collection.get(0) {
                            Some(Ok(return_value)) => {
                                return_value.raw_value().unwrap_or_else(|| {
                                    UntaggedValue::string(error_key).into_value(&name)
                                })
                            }
                            Some(Err(_)) | None => {
                                UntaggedValue::string(error_key).into_value(&name)
                            }
                        };

                        keys.push(as_string(&value));
                    }
                    Err(_) => {
                        keys.push(Ok(error_key.into()));
                    }
                }
            }

            group_strategy = Grouper::ByBlock;
        }
        Some(other) => {
            group_strategy = Grouper::ByColumn(Some(as_string(&other)?.tagged(&name)));
        }
        _ => {}
    }

    if values.is_empty() {
        return Err(ShellError::labeled_error(
            "expected table from pipeline",
            "requires a table input",
            name,
        ));
    }

    let first = values[0].clone();

    let name = if first.tag.anchor().is_some() {
        first.tag
    } else {
        name
    };

    let values = UntaggedValue::table(&values).into_value(&name);

    let group_value = match group_strategy {
        Grouper::ByBlock => {
            let map = keys.clone();

            let block = Box::new(move |idx: usize, row: &Value| match map.get(idx) {
                Some(Ok(key)) => Ok(key.clone()),
                Some(Err(reason)) => Err(reason.clone()),
                None => as_string(row),
            });

            nu_data::utils::group(&values, &Some(block), name)
        }
        Grouper::ByColumn(column_name) => group(&column_name, &values, &name),
    };

    Ok(OutputStream::one(ReturnSuccess::value(group_value?)))
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
            let block = Box::new(move |_, row: &Value| {
                match row.get_data_by_key(column_name.borrow_spanned()) {
                    Some(group_key) => Ok(as_string(&group_key)?),
                    None => Err(suggestions(column_name.borrow_tagged(), &row)),
                }
            });

            nu_data::utils::group(&values, &Some(block), &name)
        }
        Grouper::ByColumn(None) => {
            let block = Box::new(move |_, row: &Value| as_string(row));

            nu_data::utils::group(&values, &Some(block), &name)
        }
        Grouper::ByBlock => Err(ShellError::unimplemented(
            "Block not implemented: This should never happen.",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::group;
    use nu_data::utils::helpers::committers;
    use nu_errors::ShellError;
    use nu_source::*;
    use nu_test_support::value::{date, int, row, string, table};

    #[test]
    fn groups_table_by_date_column() -> Result<(), ShellError> {
        let for_key = Some(String::from("date").tagged_unknown());
        let sample = table(&committers());

        assert_eq!(
            group(&for_key, &sample, Tag::unknown())?,
            row(indexmap! {
                "2019-07-23".into() =>  table(&[
                    row(indexmap!{"name".into() => string("AR"), "country".into() => string("EC"), "date".into() => date("2019-07-23"), "chickens".into() => int(10) }),
                    row(indexmap!{"name".into() => string("JT"), "country".into() => string("NZ"), "date".into() => date("2019-07-23"), "chickens".into() =>  int(5) }),
                    row(indexmap!{"name".into() => string("YK"), "country".into() => string("US"), "date".into() => date("2019-07-23"), "chickens".into() =>  int(2) })
                ]),
                "2019-10-10".into() =>  table(&[
                    row(indexmap!{"name".into() => string("YK"), "country".into() => string("US"), "date".into() => date("2019-10-10"), "chickens".into() =>  int(6) }),
                    row(indexmap!{"name".into() => string("JT"), "country".into() => string("NZ"), "date".into() => date("2019-10-10"), "chickens".into() => int(15) }),
                    row(indexmap!{"name".into() => string("AR"), "country".into() => string("EC"), "date".into() => date("2019-10-10"), "chickens".into() => int(30) })
                ]),
                "2019-09-24".into() =>  table(&[
                    row(indexmap!{"name".into() => string("AR"), "country".into() => string("EC"), "date".into() => date("2019-09-24"), "chickens".into() => int(20) }),
                    row(indexmap!{"name".into() => string("YK"), "country".into() => string("US"), "date".into() => date("2019-09-24"), "chickens".into() =>  int(4) }),
                    row(indexmap!{"name".into() => string("JT"), "country".into() => string("NZ"), "date".into() => date("2019-09-24"), "chickens".into() => int(10) })
                ]),
            })
        );

        Ok(())
    }

    #[test]
    fn groups_table_by_country_column() -> Result<(), ShellError> {
        let for_key = Some(String::from("country").tagged_unknown());
        let sample = table(&committers());

        assert_eq!(
            group(&for_key, &sample, Tag::unknown())?,
            row(indexmap! {
                "EC".into() =>  table(&[
                    row(indexmap!{"name".into() => string("AR"), "country".into() => string("EC"), "date".into() => date("2019-07-23"), "chickens".into() => int(10) }),
                    row(indexmap!{"name".into() => string("AR"), "country".into() => string("EC"), "date".into() => date("2019-09-24"), "chickens".into() => int(20) }),
                    row(indexmap!{"name".into() => string("AR"), "country".into() => string("EC"), "date".into() => date("2019-10-10"), "chickens".into() => int(30) })
                ]),
                "NZ".into() =>  table(&[
                    row(indexmap!{"name".into() => string("JT"), "country".into() => string("NZ"), "date".into() => date("2019-07-23"), "chickens".into() =>  int(5) }),
                    row(indexmap!{"name".into() => string("JT"), "country".into() => string("NZ"), "date".into() => date("2019-10-10"), "chickens".into() => int(15) }),
                    row(indexmap!{"name".into() => string("JT"), "country".into() => string("NZ"), "date".into() => date("2019-09-24"), "chickens".into() => int(10) })
                ]),
                "US".into() =>  table(&[
                    row(indexmap!{"name".into() => string("YK"), "country".into() => string("US"), "date".into() => date("2019-10-10"), "chickens".into() =>  int(6) }),
                    row(indexmap!{"name".into() => string("YK"), "country".into() => string("US"), "date".into() => date("2019-09-24"), "chickens".into() =>  int(4) }),
                    row(indexmap!{"name".into() => string("YK"), "country".into() => string("US"), "date".into() => date("2019-07-23"), "chickens".into() =>  int(2) }),
                ]),
            })
        );

        Ok(())
    }
}
