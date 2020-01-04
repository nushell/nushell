use crate::data::TaggedListBuilder;
use chrono::{DateTime, NaiveDate, Utc};
use nu_errors::ShellError;
use nu_protocol::{Primitive, TaggedDictBuilder, UntaggedValue, Value};
use nu_source::{SpannedItem, Tag, Tagged, TaggedItem};
use nu_value_ext::{get_data_by_key, ValueExt};
use num_bigint::BigInt;
use num_traits::Zero;

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
                    _ => k.as_string().unwrap_or_else(|_| String::from("<string>")),
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
                                    }) => {
                                        dict.get_data_by_key(label.borrow_spanned()).unwrap_or_else(
                                            || UntaggedValue::Table(vec![]).into_value(&origin_tag),
                                        )
                                    }
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

pub fn fetch(key: Option<String>) -> Box<dyn Fn(Value, Tag) -> Option<Value> + 'static> {
    Box::new(move |value: Value, tag| match &key {
        Some(key_given) => value.get_data_by_key(key_given[..].spanned(tag.span)),
        None => Some(UntaggedValue::int(1).into_value(tag)),
    })
}

pub fn evaluate(
    values: &Value,
    evaluator: Option<String>,
    tag: impl Into<Tag>,
) -> Result<Value, ShellError> {
    let tag = tag.into();

    let evaluate_with = match evaluator {
        Some(keyfn) => fetch(Some(keyfn)),
        None => fetch(None),
    };

    let results: Value = match values {
        Value {
            value: UntaggedValue::Table(datasets),
            ..
        } => {
            let datasets: Vec<_> = datasets
                .iter()
                .map(|subsets| match subsets {
                    Value {
                        value: UntaggedValue::Table(subsets),
                        ..
                    } => {
                        let subsets: Vec<_> = subsets
                            .clone()
                            .into_iter()
                            .map(|data| match data {
                                Value {
                                    value: UntaggedValue::Table(data),
                                    ..
                                } => {
                                    let data: Vec<_> = data
                                        .into_iter()
                                        .map(|x| match evaluate_with(x, tag.clone()) {
                                            Some(val) => val,
                                            None => UntaggedValue::int(1).into_value(tag.clone()),
                                        })
                                        .collect();
                                    UntaggedValue::Table(data).into_value(&tag)
                                }
                                _ => UntaggedValue::Table(vec![]).into_value(&tag),
                            })
                            .collect();
                        UntaggedValue::Table(subsets).into_value(&tag)
                    }
                    _ => UntaggedValue::Table(vec![]).into_value(&tag),
                })
                .collect();

            UntaggedValue::Table(datasets).into_value(&tag)
        }
        _ => UntaggedValue::Table(vec![]).into_value(&tag),
    };

    Ok(results)
}

fn sum(data: Vec<Value>) -> Result<Value, ShellError> {
    let total = data
        .into_iter()
        .fold(Zero::zero(), |acc: BigInt, value| match value {
            Value {
                value: UntaggedValue::Primitive(Primitive::Int(n)),
                ..
            } => acc + n,
            _ => acc,
        });

    Ok(UntaggedValue::int(total).into_untagged_value())
}

fn formula(
    acc_begin: BigInt,
    calculator: Box<dyn Fn(Vec<Value>) -> Result<Value, ShellError> + 'static>,
) -> Box<dyn Fn(BigInt, Vec<Value>) -> Result<Value, ShellError> + 'static> {
    Box::new(move |acc, datax| -> Result<Value, ShellError> {
        let result = acc * acc_begin.clone();

        if let Ok(Value {
            value: UntaggedValue::Primitive(Primitive::Int(computed)),
            ..
        }) = calculator(datax)
        {
            return Ok(UntaggedValue::int(result + computed).into_untagged_value());
        }

        Ok(UntaggedValue::int(0).into_untagged_value())
    })
}

pub fn reducer_for(
    command: Reduce,
) -> Box<dyn Fn(BigInt, Vec<Value>) -> Result<Value, ShellError> + 'static> {
    match command {
        Reduce::Sum | Reduce::Default => Box::new(formula(Zero::zero(), Box::new(sum))),
    }
}

pub enum Reduce {
    Sum,
    Default,
}

pub fn reduce(
    values: &Value,
    reducer: Option<String>,
    tag: impl Into<Tag>,
) -> Result<Value, ShellError> {
    let tag = tag.into();

    let reduce_with = match reducer {
        Some(cmd) if cmd == "sum" => reducer_for(Reduce::Sum),
        Some(_) | None => reducer_for(Reduce::Default),
    };

    let results: Value = match values {
        Value {
            value: UntaggedValue::Table(datasets),
            ..
        } => {
            let datasets: Vec<_> = datasets
                .iter()
                .map(|subsets| {
                    let acc: BigInt = Zero::zero();
                    match subsets {
                        Value {
                            value: UntaggedValue::Table(data),
                            ..
                        } => {
                            let data = data
                                .iter()
                                .map(|d| {
                                    if let Value {
                                        value: UntaggedValue::Table(x),
                                        ..
                                    } = d
                                    {
                                        if let Ok(Value {
                                            value:
                                                UntaggedValue::Primitive(Primitive::Int(computed)),
                                            ..
                                        }) = reduce_with(acc.clone(), x.clone())
                                        {
                                            UntaggedValue::int(computed).into_value(&tag)
                                        } else {
                                            UntaggedValue::int(0).into_value(&tag)
                                        }
                                    } else {
                                        UntaggedValue::int(0).into_value(&tag)
                                    }
                                })
                                .collect::<Vec<_>>();
                            UntaggedValue::Table(data).into_value(&tag)
                        }
                        _ => UntaggedValue::Table(vec![]).into_value(&tag),
                    }
                })
                .collect();

            UntaggedValue::Table(datasets).into_value(&tag)
        }
        _ => UntaggedValue::Table(vec![]).into_value(&tag),
    };

    Ok(results)
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
                .iter()
                .map(|subsets| match subsets {
                    Value {
                        value: UntaggedValue::Table(data),
                        ..
                    } => {
                        let data: BigInt =
                            data.iter().fold(Zero::zero(), |acc, value| match value {
                                Value {
                                    value: UntaggedValue::Primitive(Primitive::Int(n)),
                                    ..
                                } if *n > acc => n.clone(),
                                _ => acc,
                            });
                        UntaggedValue::int(data).into_value(&tag)
                    }
                    _ => UntaggedValue::int(0).into_value(&tag),
                })
                .collect();

            let datasets: BigInt = datasets
                .iter()
                .fold(Zero::zero(), |max, value| match value {
                    Value {
                        value: UntaggedValue::Primitive(Primitive::Int(n)),
                        ..
                    } if *n > max => n.clone(),
                    _ => max,
                });
            UntaggedValue::int(datasets).into_value(&tag)
        }
        _ => UntaggedValue::int(-1).into_value(&tag),
    };

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::{columns_sorted, evaluate, fetch, map_max, reduce, reducer_for, t_sort, Reduce};
    use crate::commands::group_by::group;
    use indexmap::IndexMap;
    use nu_errors::ShellError;
    use nu_protocol::{UntaggedValue, Value};
    use nu_source::*;
    use num_bigint::BigInt;
    use num_traits::Zero;

    fn int(s: impl Into<BigInt>) -> Value {
        UntaggedValue::int(s).into_untagged_value()
    }

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

    fn nu_releases_sorted_by_date() -> Result<Value, ShellError> {
        let key = String::from("date");

        t_sort(
            Some(key),
            None,
            &nu_releases_grouped_by_date()?,
            Tag::unknown(),
        )
    }

    fn nu_releases_evaluated_by_default_one() -> Result<Value, ShellError> {
        evaluate(&nu_releases_sorted_by_date()?, None, Tag::unknown())
    }

    fn nu_releases_reduced_by_sum() -> Result<Value, ShellError> {
        reduce(
            &nu_releases_evaluated_by_default_one()?,
            Some(String::from("sum")),
            Tag::unknown(),
        )
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
    fn show_columns_sorted_given_a_column_to_sort_by() -> Result<(), ShellError> {
        let by_column = String::from("date");

        assert_eq!(
            columns_sorted(
                Some(by_column),
                &nu_releases_grouped_by_date()?,
                Tag::unknown()
            ),
            vec![
                "August 23-2019".to_string().tagged_unknown(),
                "September 24-2019".to_string().tagged_unknown(),
                "October 10-2019".to_string().tagged_unknown()
            ]
        );

        Ok(())
    }

    #[test]
    fn sorts_the_tables() -> Result<(), ShellError> {
        let group_by = String::from("date");

        assert_eq!(
            t_sort(
                Some(group_by),
                None,
                &nu_releases_grouped_by_date()?,
                Tag::unknown()
            )?,
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

        Ok(())
    }

    #[test]
    fn evaluator_fetches_by_column_if_supplied_a_column_name() -> Result<(), ShellError> {
        let subject = row(indexmap! { "name".into() => string("andres") });

        let evaluator = fetch(Some(String::from("name")));

        assert_eq!(evaluator(subject, Tag::unknown()), Some(string("andres")));
        Ok(())
    }

    #[test]
    fn evaluator_returns_1_if_no_column_name_given() -> Result<(), ShellError> {
        let subject = row(indexmap! { "name".into() => string("andres") });
        let evaluator = fetch(None);

        assert_eq!(
            evaluator(subject, Tag::unknown()),
            Some(UntaggedValue::int(1).into_untagged_value())
        );

        Ok(())
    }

    #[test]
    fn evaluates_the_tables() -> Result<(), ShellError> {
        assert_eq!(
            evaluate(&nu_releases_sorted_by_date()?, None, Tag::unknown())?,
            table(&[table(&[
                table(&[int(1), int(1), int(1)]),
                table(&[int(1), int(1), int(1)]),
                table(&[int(1), int(1), int(1)]),
            ]),])
        );

        Ok(())
    }

    #[test]
    fn evaluates_the_tables_with_custom_evaluator() -> Result<(), ShellError> {
        let eval = String::from("name");

        assert_eq!(
            evaluate(&nu_releases_sorted_by_date()?, Some(eval), Tag::unknown())?,
            table(&[table(&[
                table(&[string("AR"), string("JT"), string("YK")]),
                table(&[string("AR"), string("YK"), string("JT")]),
                table(&[string("YK"), string("JT"), string("AR")]),
            ]),])
        );

        Ok(())
    }

    #[test]
    fn reducer_computes_given_a_sum_command() -> Result<(), ShellError> {
        let subject = vec![int(1), int(1), int(1)];

        let action = reducer_for(Reduce::Sum);

        assert_eq!(action(Zero::zero(), subject)?, int(3));

        Ok(())
    }

    #[test]
    fn reducer_computes() -> Result<(), ShellError> {
        assert_eq!(
            reduce(
                &nu_releases_evaluated_by_default_one()?,
                Some(String::from("sum")),
                Tag::unknown()
            )?,
            table(&[table(&[int(3), int(3), int(3)])])
        );

        Ok(())
    }

    #[test]
    fn maps_and_gets_max_value() -> Result<(), ShellError> {
        assert_eq!(
            map_max(&nu_releases_reduced_by_sum()?, None, Tag::unknown())?,
            int(3)
        );

        Ok(())
    }
}
