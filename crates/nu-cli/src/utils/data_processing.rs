use crate::data::value::compare_values;
use crate::data::TaggedListBuilder;
use chrono::{DateTime, NaiveDate, Utc};
use nu_errors::ShellError;
use nu_protocol::hir::Operator;
use nu_protocol::{Primitive, TaggedDictBuilder, UntaggedValue, Value};
use nu_source::{SpannedItem, Tag, Tagged, TaggedItem};
use nu_value_ext::{get_data_by_key, ValueExt};
use num_traits::Zero;

// Re-usable error messages
const ERR_EMPTY_DATA: &str = "Cannot perform aggregate math operation on empty data";

pub fn columns_sorted(
    _group_by_name: Option<Tagged<String>>,
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
                    let date = NaiveDate::parse_from_str(k, "%Y-%m-%d");

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
                    } => format!("{}", d.format("%Y-%m-%d")),
                    _ => k.as_string().unwrap_or_else(|_| String::from("<string>")),
                })
                .collect();

            keys.into_iter().map(|k| k.tagged(&origin_tag)).collect()
        }
        _ => vec!["default".to_owned().tagged(&origin_tag)],
    }
}

pub fn t_sort(
    group_by_name: Option<Tagged<String>>,
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

pub fn sum(data: Vec<Value>) -> Result<Value, ShellError> {
    if data.is_empty() {
        return Err(ShellError::unexpected(ERR_EMPTY_DATA));
    }
    let mut acc = Value::zero();
    for value in data {
        match value.value {
            UntaggedValue::Primitive(_) => acc = acc + value,
            _ => {
                return Err(ShellError::labeled_error(
                    "Attempted to compute the sum of a value that cannot be summed.",
                    "value appears here",
                    value.tag.span,
                ))
            }
        }
    }
    Ok(acc)
}

pub fn max(data: Vec<Value>) -> Result<Value, ShellError> {
    let mut biggest = data
        .first()
        .ok_or_else(|| ShellError::unexpected(ERR_EMPTY_DATA))?
        .value
        .clone();

    for value in data.iter() {
        if let Ok(greater_than) = compare_values(Operator::GreaterThan, &value.value, &biggest) {
            if greater_than {
                biggest = value.value.clone();
            }
        } else {
            return Err(ShellError::unexpected(format!(
                "Could not compare\nleft: {:?}\nright: {:?}",
                biggest, value.value
            )));
        }
    }
    Ok(Value {
        value: biggest,
        tag: Tag::unknown(),
    })
}

pub fn min(data: Vec<Value>) -> Result<Value, ShellError> {
    let mut smallest = data
        .first()
        .ok_or_else(|| ShellError::unexpected(ERR_EMPTY_DATA))?
        .value
        .clone();

    for value in data.iter() {
        if let Ok(greater_than) = compare_values(Operator::LessThan, &value.value, &smallest) {
            if greater_than {
                smallest = value.value.clone();
            }
        } else {
            return Err(ShellError::unexpected(format!(
                "Could not compare\nleft: {:?}\nright: {:?}",
                smallest, value.value
            )));
        }
    }
    Ok(Value {
        value: smallest,
        tag: Tag::unknown(),
    })
}

fn formula(
    acc_begin: Value,
    calculator: Box<dyn Fn(Vec<Value>) -> Result<Value, ShellError> + Send + Sync + 'static>,
) -> Box<dyn Fn(Value, Vec<Value>) -> Result<Value, ShellError> + Send + Sync + 'static> {
    Box::new(move |acc, datax| -> Result<Value, ShellError> {
        let result = acc * acc_begin.clone();

        match calculator(datax) {
            Ok(total) => Ok(result + total),
            Err(reason) => Err(reason),
        }
    })
}

pub fn reducer_for(
    command: Reduce,
) -> Box<dyn Fn(Value, Vec<Value>) -> Result<Value, ShellError> + Send + Sync + 'static> {
    match command {
        Reduce::Summation | Reduce::Default => Box::new(formula(Value::zero(), Box::new(sum))),
        Reduce::Minimum => Box::new(|_, values| min(values)),
        Reduce::Maximum => Box::new(|_, values| max(values)),
    }
}

pub enum Reduce {
    Summation,
    Minimum,
    Maximum,
    Default,
}

pub fn reduce(
    values: &Value,
    reducer: Option<String>,
    tag: impl Into<Tag>,
) -> Result<Value, ShellError> {
    let tag = tag.into();

    let reduce_with = match reducer {
        Some(cmd) if cmd == "sum" => reducer_for(Reduce::Summation),
        Some(cmd) if cmd == "min" => reducer_for(Reduce::Minimum),
        Some(cmd) if cmd == "max" => reducer_for(Reduce::Maximum),
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
                    let acc = Value::zero();
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
            let datasets: Vec<Value> = datasets
                .iter()
                .map(|subsets| match subsets {
                    Value {
                        value: UntaggedValue::Table(data),
                        ..
                    } => data.iter().fold(Value::zero(), |acc, value| {
                        let left = &value.value;
                        let right = &acc.value;

                        if let Ok(is_greater_than) =
                            compare_values(Operator::GreaterThan, left, right)
                        {
                            if is_greater_than {
                                value.clone()
                            } else {
                                acc
                            }
                        } else {
                            acc
                        }
                    }),
                    _ => UntaggedValue::int(0).into_value(&tag),
                })
                .collect();

            datasets.into_iter().fold(Value::zero(), |max, value| {
                let left = &value.value;
                let right = &max.value;

                if let Ok(is_greater_than) = compare_values(Operator::GreaterThan, left, right) {
                    if is_greater_than {
                        value
                    } else {
                        max
                    }
                } else {
                    max
                }
            })
        }
        _ => UntaggedValue::int(-1).into_value(&tag),
    };

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::{columns_sorted, evaluate, fetch, map_max, reduce, reducer_for, t_sort, Reduce};
    use indexmap::IndexMap;
    use nu_protocol::{UntaggedValue, Value};
    use nu_source::*;
    use num_bigint::BigInt;
    use num_traits::Zero;

    fn int(s: impl Into<BigInt>) -> Value {
        crate::utils::data::tests::helpers::int(s)
    }

    fn string(input: impl Into<String>) -> Value {
        crate::utils::data::tests::helpers::string(input)
    }

    pub fn date(input: impl Into<String>) -> Value {
        crate::utils::data::tests::helpers::date(input)
    }

    fn row(entries: IndexMap<String, Value>) -> Value {
        crate::utils::data::tests::helpers::row(entries)
    }

    fn table(list: &[Value]) -> Value {
        crate::utils::data::tests::helpers::table(list)
    }

    fn committers_grouped_by_date() -> Value {
        crate::utils::data::tests::helpers::committers_grouped_by_date()
    }

    fn datasets_sorted_by_date() -> Value {
        crate::utils::data::tests::helpers::datasets_sorted_by_date()
    }

    fn datasets_evaluated_by_default_one() -> Value {
        crate::utils::data::tests::helpers::datasets_evaluated_by_default_one()
    }

    fn nu_releases_reduced_by_sum() -> Value {
        reduce(
            &datasets_evaluated_by_default_one(),
            Some(String::from("sum")),
            Tag::unknown(),
        )
        .unwrap()
    }

    #[test]
    fn show_columns_sorted_given_a_column_to_sort_by() {
        let by_column = String::from("date").tagged(Tag::unknown());

        assert_eq!(
            columns_sorted(
                Some(by_column),
                &committers_grouped_by_date(),
                Tag::unknown()
            ),
            vec![
                "2019-07-23".to_string().tagged_unknown(),
                "2019-09-24".to_string().tagged_unknown(),
                "2019-10-10".to_string().tagged_unknown()
            ]
        );
    }

    #[test]
    fn sorts_the_tables() {
        let group_by = String::from("date").tagged(Tag::unknown());

        assert_eq!(
            t_sort(
                Some(group_by),
                None,
                &committers_grouped_by_date(),
                Tag::unknown()
            )
            .unwrap(),
            table(&[table(&[
                table(&[
                    row(
                        indexmap! {"name".into() => string("AR"), "country".into() => string("EC"), "date".into() => date("2019-07-23"), "chickens".into() => int(10) }
                    ),
                    row(
                        indexmap! {"name".into() => string("JT"), "country".into() => string("NZ"), "date".into() => date("2019-07-23"), "chickens".into() =>  int(5) }
                    ),
                    row(
                        indexmap! {"name".into() => string("YK"), "country".into() => string("US"), "date".into() => date("2019-07-23"), "chickens".into() =>  int(2) }
                    )
                ]),
                table(&[
                    row(
                        indexmap! {"name".into() => string("AR"), "country".into() => string("EC"), "date".into() => date("2019-09-24"), "chickens".into() => int(20) }
                    ),
                    row(
                        indexmap! {"name".into() => string("YK"), "country".into() => string("US"), "date".into() => date("2019-09-24"), "chickens".into() =>  int(4) }
                    ),
                    row(
                        indexmap! {"name".into() => string("JT"), "country".into() => string("NZ"), "date".into() => date("2019-09-24"), "chickens".into() => int(10) }
                    )
                ]),
                table(&[
                    row(
                        indexmap! {"name".into() => string("YK"), "country".into() => string("US"), "date".into() => date("2019-10-10"), "chickens".into() =>  int(6) }
                    ),
                    row(
                        indexmap! {"name".into() => string("JT"), "country".into() => string("NZ"), "date".into() => date("2019-10-10"), "chickens".into() => int(15) }
                    ),
                    row(
                        indexmap! {"name".into() => string("AR"), "country".into() => string("EC"), "date".into() => date("2019-10-10"), "chickens".into() => int(30) }
                    )
                ]),
            ]),])
        );
    }

    #[test]
    fn evaluator_fetches_by_column_if_supplied_a_column_name() {
        let subject = row(indexmap! { "name".into() => string("andres") });

        let evaluator = fetch(Some(String::from("name")));

        assert_eq!(evaluator(subject, Tag::unknown()), Some(string("andres")));
    }

    #[test]
    fn evaluator_returns_1_if_no_column_name_given() {
        let subject = row(indexmap! { "name".into() => string("andres") });
        let evaluator = fetch(None);

        assert_eq!(
            evaluator(subject, Tag::unknown()),
            Some(UntaggedValue::int(1).into_untagged_value())
        );
    }

    #[test]
    fn evaluates_the_tables() {
        assert_eq!(
            evaluate(&datasets_sorted_by_date(), None, Tag::unknown()).unwrap(),
            table(&[table(&[
                table(&[int(1), int(1), int(1)]),
                table(&[int(1), int(1), int(1)]),
                table(&[int(1), int(1), int(1)]),
            ]),])
        );
    }

    #[test]
    fn evaluates_the_tables_with_custom_evaluator() {
        let eval = String::from("name");

        assert_eq!(
            evaluate(&datasets_sorted_by_date(), Some(eval), Tag::unknown()).unwrap(),
            table(&[table(&[
                table(&[string("AR"), string("JT"), string("YK")]),
                table(&[string("AR"), string("YK"), string("JT")]),
                table(&[string("YK"), string("JT"), string("AR")]),
            ]),])
        );
    }

    #[test]
    fn reducer_computes_given_a_sum_command() {
        let subject = vec![int(1), int(1), int(1)];

        let action = reducer_for(Reduce::Summation);

        assert_eq!(action(Value::zero(), subject).unwrap(), int(3));
    }

    #[test]
    fn reducer_computes() {
        assert_eq!(
            reduce(
                &datasets_evaluated_by_default_one(),
                Some(String::from("sum")),
                Tag::unknown()
            )
            .unwrap(),
            table(&[int(3), int(3), int(3)])
        );
    }

    #[test]
    fn maps_and_gets_max_value() {
        assert_eq!(
            map_max(&nu_releases_reduced_by_sum(), None, Tag::unknown()).unwrap(),
            int(3)
        );
    }
}
