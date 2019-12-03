use crate::commands::WholeStreamCommand;
use crate::data::value;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;
use num_traits::cast::ToPrimitive;

pub struct MapMaxBy;

#[derive(Deserialize)]
pub struct MapMaxByArgs {
    column_name: Option<Tagged<String>>,
}

impl WholeStreamCommand for MapMaxBy {
    fn name(&self) -> &str {
        "map-max-by"
    }

    fn signature(&self) -> Signature {
        Signature::build("map-max-by").named(
            "column_name",
            SyntaxShape::String,
            "the name of the column to map-max the table's rows",
        )
    }

    fn usage(&self) -> &str {
        "Creates a new table with the data from the tables rows maxed by the column given."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, map_max_by)?.run()
    }
}

pub fn map_max_by(
    MapMaxByArgs { column_name }: MapMaxByArgs,
    RunnableContext { input, name, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let stream = async_stream! {
        let values: Vec<Value> = input.values.collect().await;


        if values.is_empty() {
            yield Err(ShellError::labeled_error(
                    "Expected table from pipeline",
                    "requires a table input",
                    name
                ))
        } else {

            let map_by_column = if let Some(column_to_map) = column_name {
                Some(column_to_map.item().clone())
            } else {
                None
            };

            match map_max(&values[0], map_by_column, name) {
                Ok(table_maxed) => yield ReturnSuccess::value(table_maxed),
                Err(err) => yield Err(err)
            }
        }
    };

    Ok(stream.to_output_stream())
}

pub fn map_max(
    values: &Value,
    _map_by_column_name: Option<String>,
    tag: impl Into<Tag>,
) -> Result<Value, ShellError> {
    let tag = tag.into();

    let results: Value = match values {
        Value {
            value: UntaggedValue::Table(datasets),
            ..
        } => {
            let datasets: Vec<_> = datasets
                .into_iter()
                .map(|subsets| match subsets {
                    Value {
                        value: UntaggedValue::Table(data),
                        ..
                    } => {
                        let data = data.into_iter().fold(0, |acc, value| match value {
                            Value {
                                value: UntaggedValue::Primitive(Primitive::Int(n)),
                                ..
                            } => {
                                if n.to_i32().unwrap() > acc {
                                    n.to_i32().unwrap()
                                } else {
                                    acc
                                }
                            }
                            _ => acc,
                        });
                        value::number(data).into_value(&tag)
                    }
                    _ => value::number(0).into_value(&tag),
                })
                .collect();

            let datasets = datasets.iter().fold(0, |max, value| match value {
                Value {
                    value: UntaggedValue::Primitive(Primitive::Int(n)),
                    ..
                } => {
                    if n.to_i32().unwrap() > max {
                        n.to_i32().unwrap()
                    } else {
                        max
                    }
                }
                _ => max,
            });
            value::number(datasets).into_value(&tag)
        }
        _ => value::number(-1).into_value(&tag),
    };

    Ok(results)
}

#[cfg(test)]
mod tests {

    use crate::commands::evaluate_by::evaluate;
    use crate::commands::group_by::group;
    use crate::commands::map_max_by::map_max;
    use crate::commands::reduce_by::reduce;
    use crate::commands::t_sort_by::t_sort;
    use crate::prelude::*;
    use indexmap::IndexMap;
    use nu_protocol::{UntaggedValue, Value};
    use nu_source::*;

    fn int(s: impl Into<BigInt>) -> Value {
        value::int(s).into_untagged_value()
    }

    fn string(input: impl Into<String>) -> Value {
        value::string(input.into()).into_untagged_value()
    }

    fn row(entries: IndexMap<String, Value>) -> Value {
        value::row(entries).into_untagged_value()
    }

    fn nu_releases_evaluated_by_default_one() -> Value {
        evaluate(&nu_releases_sorted_by_date(), None, Tag::unknown()).unwrap()
    }

    fn nu_releases_reduced_by_sum() -> Value {
        reduce(
            &nu_releases_evaluated_by_default_one(),
            Some(String::from("sum")),
            Tag::unknown(),
        )
        .unwrap()
    }

    fn nu_releases_sorted_by_date() -> Value {
        let key = String::from("date");

        t_sort(
            Some(key),
            None,
            &nu_releases_grouped_by_date(),
            Tag::unknown(),
        )
        .unwrap()
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
            row(
                indexmap! {"name".into() => string("JK"), "country".into() => string("US"), "date".into() => string("August 23-2019")},
            ),
        ]
    }
    #[test]
    fn maps_and_gets_max_value() {
        assert_eq!(
            map_max(&nu_releases_reduced_by_sum(), None, Tag::unknown()).unwrap(),
            int(4)
        );
    }
}
