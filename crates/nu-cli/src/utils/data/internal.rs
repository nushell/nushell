#![allow(clippy::type_complexity)]

use crate::data::value::compute_values;
use derive_new::new;
use nu_errors::ShellError;
use nu_protocol::hir::Operator;
use nu_protocol::{UntaggedValue, Value};
use nu_source::{SpannedItem, Tag, TaggedItem};
use nu_value_ext::ValueExt;

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone, new)]
pub struct Labels {
    pub x: Vec<String>,
    pub y: Vec<String>,
}

impl Labels {
    pub fn at(&self, idx: usize) -> Option<&str> {
        if let Some(k) = self.x.get(idx) {
            Some(&k[..])
        } else {
            None
        }
    }

    pub fn grouped(&self) -> impl Iterator<Item = &String> {
        self.x.iter()
    }

    pub fn grouping_total(&self) -> Value {
        UntaggedValue::int(self.x.len()).into_untagged_value()
    }

    pub fn splits(&self) -> impl Iterator<Item = &String> {
        self.y.iter()
    }

    pub fn splits_total(&self) -> Value {
        UntaggedValue::int(self.y.len()).into_untagged_value()
    }
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone, new)]
pub struct Range {
    pub start: Value,
    pub end: Value,
}

fn formula(
    acc_begin: Value,
    calculator: Box<dyn Fn(Vec<&Value>) -> Result<Value, ShellError> + Send + Sync + 'static>,
) -> Box<dyn Fn(&Value, Vec<&Value>) -> Result<Value, ShellError> + Send + Sync + 'static> {
    Box::new(move |acc, datax| -> Result<Value, ShellError> {
        let result = match compute_values(Operator::Multiply, &acc, &acc_begin) {
            Ok(v) => v.into_untagged_value(),
            Err((left_type, right_type)) => {
                return Err(ShellError::coerce_error(
                    left_type.spanned_unknown(),
                    right_type.spanned_unknown(),
                ))
            }
        };

        match calculator(datax) {
            Ok(total) => Ok(match compute_values(Operator::Plus, &result, &total) {
                Ok(v) => v.into_untagged_value(),
                Err((left_type, right_type)) => {
                    return Err(ShellError::coerce_error(
                        left_type.spanned_unknown(),
                        right_type.spanned_unknown(),
                    ))
                }
            }),
            Err(reason) => Err(reason),
        }
    })
}

pub fn reducer_for(
    command: Reduction,
) -> Box<dyn Fn(&Value, Vec<&Value>) -> Result<Value, ShellError> + Send + Sync + 'static> {
    match command {
        Reduction::Accumulate => Box::new(formula(
            UntaggedValue::int(1).into_untagged_value(),
            Box::new(sum),
        )),
        _ => Box::new(formula(
            UntaggedValue::int(0).into_untagged_value(),
            Box::new(sum),
        )),
    }
}

pub fn max(values: &Value, tag: impl Into<Tag>) -> Result<&Value, ShellError> {
    let tag = tag.into();

    values
        .table_entries()
        .filter_map(|dataset| dataset.table_entries().max())
        .max()
        .ok_or_else(|| ShellError::labeled_error("err", "err", &tag))
}

pub fn sum(data: Vec<&Value>) -> Result<Value, ShellError> {
    let mut acc = UntaggedValue::int(0);

    for value in data {
        match value.value {
            UntaggedValue::Primitive(_) => {
                acc = match compute_values(Operator::Plus, &acc, &value) {
                    Ok(v) => v,
                    Err((left_type, right_type)) => {
                        return Err(ShellError::coerce_error(
                            left_type.spanned_unknown(),
                            right_type.spanned_unknown(),
                        ))
                    }
                };
            }
            _ => {
                return Err(ShellError::labeled_error(
                    "Attempted to compute the sum of a value that cannot be summed.",
                    "value appears here",
                    value.tag.span,
                ))
            }
        }
    }
    Ok(acc.into_untagged_value())
}

pub fn sort_columns(
    values: &[String],
    format: &Option<Box<dyn Fn(&Value, String) -> Result<String, ShellError>>>,
) -> Result<Vec<String>, ShellError> {
    let mut keys = vec![];

    if let Some(fmt) = format {
        for k in values.iter() {
            let k = k.clone().tagged_unknown();
            let v =
                crate::data::value::Date::naive_from_str(k.borrow_tagged())?.into_untagged_value();
            keys.push(fmt(&v, k.to_string())?);
        }
    } else {
        keys = values.to_vec();
    }

    keys.sort();
    Ok(keys)
}

pub fn sort(planes: &Labels, values: &Value, tag: impl Into<Tag>) -> Result<Value, ShellError> {
    let tag = tag.into();

    let mut x = vec![];
    for column in planes.splits() {
        let key = column.clone().tagged_unknown();
        let groups = values
            .get_data_by_key(key.borrow_spanned())
            .ok_or_else(|| {
                ShellError::labeled_error("unknown column", "unknown column", key.span())
            })?;

        let mut y = vec![];
        for inner_column in planes.grouped() {
            let key = inner_column.clone().tagged_unknown();
            let grouped = groups.get_data_by_key(key.borrow_spanned());

            if let Some(grouped) = grouped {
                y.push(grouped.table_entries().cloned().collect::<Vec<_>>());
            } else {
                let empty = UntaggedValue::table(&[]).into_value(&tag);
                y.push(empty.table_entries().cloned().collect::<Vec<_>>());
            }
        }

        x.push(
            UntaggedValue::table(&y.iter().cloned().flatten().collect::<Vec<Value>>())
                .into_value(&tag),
        );
    }

    Ok(UntaggedValue::table(&x).into_value(&tag))
}

pub fn evaluate(
    values: &Value,
    evaluator: &Option<Box<dyn Fn(usize, &Value) -> Result<Value, ShellError> + Send>>,
    tag: impl Into<Tag>,
) -> Result<Value, ShellError> {
    let tag = tag.into();

    let mut x = vec![];
    for split in values.table_entries() {
        let mut y = vec![];

        for (idx, subset) in split.table_entries().enumerate() {
            let mut set = vec![];

            if let Some(ref evaluator) = evaluator {
                let value = evaluator(idx, subset)?;

                set.push(value);
            } else {
                set.push(UntaggedValue::int(1).into_value(&tag));
            }

            y.push(UntaggedValue::table(&set).into_value(&tag));
        }

        x.push(UntaggedValue::table(&y).into_value(&tag));
    }

    Ok(UntaggedValue::table(&x).into_value(&tag))
}

pub enum Reduction {
    #[allow(dead_code)]
    Count,
    Accumulate,
}

pub fn reduce(values: &Value, tag: impl Into<Tag>) -> Result<Value, ShellError> {
    let tag = tag.into();
    let reduce_with = reducer_for(Reduction::Accumulate);

    let mut datasets = vec![];
    for dataset in values.table_entries() {
        let mut acc = UntaggedValue::int(0).into_value(&tag);

        let mut subsets = vec![];
        for subset in dataset.table_entries() {
            acc = reduce_with(&acc, subset.table_entries().collect::<Vec<_>>())?;
            subsets.push(acc.clone());
        }
        datasets.push(UntaggedValue::table(&subsets).into_value(&tag));
    }

    Ok(UntaggedValue::table(&datasets).into_value(&tag))
}

pub fn percentages(
    maxima: &Value,
    values: &Value,
    tag: impl Into<Tag>,
) -> Result<Value, ShellError> {
    let tag = tag.into();

    let mut x = vec![];
    for split in values.table_entries() {
        x.push(
            UntaggedValue::table(
                &split
                    .table_entries()
                    .filter_map(|s| {
                        let hundred = UntaggedValue::decimal(100);

                        match compute_values(Operator::Divide, &hundred, &maxima) {
                            Ok(v) => match compute_values(Operator::Multiply, &s, &v) {
                                Ok(v) => Some(v.into_untagged_value()),
                                Err(_) => None,
                            },
                            Err(_) => None,
                        }
                    })
                    .collect::<Vec<_>>(),
            )
            .into_value(&tag),
        );
    }

    Ok(UntaggedValue::table(&x).into_value(&tag))
}
