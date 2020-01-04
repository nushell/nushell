use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{
    ReturnSuccess, Signature, SpannedTypeName, SyntaxShape, TaggedDictBuilder, UntaggedValue, Value,
};
use nu_source::Tagged;

pub struct SplitBy;

#[derive(Deserialize)]
pub struct SplitByArgs {
    column_name: Tagged<String>,
}

impl WholeStreamCommand for SplitBy {
    fn name(&self) -> &str {
        "split-by"
    }

    fn signature(&self) -> Signature {
        Signature::build("split-by").required(
            "column_name",
            SyntaxShape::String,
            "the name of the column within the nested table to split by",
        )
    }

    fn usage(&self) -> &str {
        "Creates a new table with the data from the inner tables splitted by the column given."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, split_by)?.run()
    }
}

pub fn split_by(
    SplitByArgs { column_name }: SplitByArgs,
    RunnableContext { input, name, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let stream = async_stream! {
        let values: Vec<Value> = input.values.collect().await;

        if values.len() > 1 || values.is_empty() {
            yield Err(ShellError::labeled_error(
                    "Expected table from pipeline",
                    "requires a table input",
                    column_name.span()
                ))
        } else {
            match split(&column_name, &values[0], name) {
                Ok(split) => yield ReturnSuccess::value(split),
                Err(err) => yield Err(err),
            }
        }
    };

    Ok(stream.to_output_stream())
}

pub fn split(
    column_name: &Tagged<String>,
    value: &Value,
    tag: impl Into<Tag>,
) -> Result<Value, ShellError> {
    let origin_tag = tag.into();

    let mut splits = indexmap::IndexMap::new();

    match value {
        Value {
            value: UntaggedValue::Row(group_sets),
            ..
        } => {
            for (group_key, group_value) in group_sets.entries.iter() {
                match *group_value {
                    Value {
                        value: UntaggedValue::Table(ref dataset),
                        ..
                    } => {
                        let group = crate::commands::group_by::group(
                            &column_name,
                            dataset.to_vec(),
                            &origin_tag,
                        )?;

                        match group {
                            Value {
                                value: UntaggedValue::Row(o),
                                ..
                            } => {
                                for (split_label, subset) in o.entries.into_iter() {
                                    match subset {
                                        Value {
                                            value: UntaggedValue::Table(subset),
                                            tag,
                                        } => {
                                            let s = splits
                                                .entry(split_label.clone())
                                                .or_insert(indexmap::IndexMap::new());
                                            s.insert(
                                                group_key.clone(),
                                                UntaggedValue::table(&subset).into_value(tag),
                                            );
                                        }
                                        other => {
                                            return Err(ShellError::type_error(
                                                "a table value",
                                                other.spanned_type_name(),
                                            ))
                                        }
                                    }
                                }
                            }
                            _ => {
                                return Err(ShellError::type_error(
                                    "a table value",
                                    group.spanned_type_name(),
                                ))
                            }
                        }
                    }
                    ref other => {
                        return Err(ShellError::type_error(
                            "a table value",
                            other.spanned_type_name(),
                        ))
                    }
                }
            }
        }
        _ => {
            return Err(ShellError::type_error(
                "a table value",
                value.spanned_type_name(),
            ))
        }
    }

    let mut out = TaggedDictBuilder::new(&origin_tag);

    for (k, v) in splits.into_iter() {
        out.insert_untagged(k, UntaggedValue::row(v));
    }

    Ok(out.into_value())
}
#[cfg(test)]
mod tests {

    use crate::commands::group_by::group;
    use crate::commands::split_by::split;
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
        let key = String::from("date").tagged_unknown();
        group(&key, nu_releases_commiters(), Tag::unknown())
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
    fn splits_inner_tables_by_key() -> Result<(), ShellError> {
        let for_key = String::from("country").tagged_unknown();

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
        let for_key = String::from("country").tagged_unknown();

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
}
