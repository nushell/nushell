use crate::data::value::compute_values;
use derive_new::new;
use getset::Getters;
use nu_errors::ShellError;
use nu_protocol::hir::Operator;
use nu_protocol::{UntaggedValue, Value};
use nu_source::{Tag, TaggedItem};
use nu_value_ext::ValueExt;
use num_traits::Zero;

// Re-usable error messages
const ERR_EMPTY_DATA: &str = "Cannot perform aggregate math operation on empty data";

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

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Getters, Clone, new)]
pub struct Range {
    #[get = "pub"]
    start: Value,
    end: Value,
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Getters, Clone, new)]
pub struct Model {
    pub labels: Labels,
    pub ranges: (Range, Range),

    pub data: Value,
    pub percentages: Value,
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Getters, Clone, new)]
pub struct Labels {
    #[get = "pub"]
    x: Vec<String>,
    y: Vec<String>,
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

pub enum Reduction {
    Count,
    Accumulate,
}

pub fn sum(data: Vec<&Value>) -> Result<Value, ShellError> {
    let mut acc = UntaggedValue::int(0);

    for value in data {
        match value.value {
            UntaggedValue::Primitive(_) => {
                acc = compute_values(Operator::Plus, &acc, &value.value).unwrap()
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

fn formula(
    acc_begin: Value,
    calculator: Box<dyn Fn(Vec<&Value>) -> Result<Value, ShellError> + Send + Sync + 'static>,
) -> Box<dyn Fn(&Value, Vec<&Value>) -> Result<Value, ShellError> + Send + Sync + 'static> {
    Box::new(move |acc, datax| -> Result<Value, ShellError> {
        let result = compute_values(Operator::Multiply, &acc, &acc_begin).unwrap();

        match calculator(datax) {
            Ok(total) => Ok(compute_values(Operator::Plus, &result, &total)
                .unwrap()
                .into_untagged_value()),
            Err(reason) => Err(reason),
        }
    })
}

pub fn reducer_for(
    command: Reduction,
) -> Box<dyn Fn(&Value, Vec<&Value>) -> Result<Value, ShellError> + Send + Sync + 'static> {
    match command {
        Reduction::Count => Box::new(formula(
            UntaggedValue::int(0).into_untagged_value(),
            Box::new(sum),
        )),
        Reduction::Accumulate => Box::new(formula(
            UntaggedValue::int(1).into_untagged_value(),
            Box::new(sum),
        )),
    }
}
//open ../../ecuacovid/datos_crudos/defunciones/provincias.csv | where provincia == "Guayas" || provincia == "Pichincha" | first 10 | chart provincia --use total
pub fn reduce(values: &Value, tag: impl Into<Tag>) -> Result<Value, ShellError> {
    let tag = tag.into();
    let reduce_with = reducer_for(Reduction::Count);

    let mut datasets = vec![];
    for dataset in values.table_entries() {
        let mut acc = UntaggedValue::int(0).into_value(&tag);

        let mut subsets = vec![];
        for subset in dataset.table_entries() {
            //for d in subset.table_entries() {
            acc = reduce_with(&acc, subset.table_entries().collect::<Vec<_>>())?;
            subsets.push(acc.clone());
            //}
        }
        datasets.push(UntaggedValue::table(&subsets).into_value(&tag));
        //datasets.push(UntaggedValue::table(&subsets).into_value(&tag));
    }

    Ok(UntaggedValue::table(&datasets).into_value(&tag))
}

pub fn max(values: &Value, tag: impl Into<Tag>) -> Result<&Value, ShellError> {
    let tag = tag.into();

    values
        .table_entries()
        .filter_map(|dataset| dataset.table_entries().max())
        .max()
        .ok_or_else(|| ShellError::labeled_error("err", "err", &tag))
}

pub struct Operation<'a> {
    pub grouper: Option<Box<dyn Fn(usize, &Value) -> Result<String, ShellError> + Send>>,
    pub splitter: Option<Box<dyn Fn(usize, &Value) -> Result<String, ShellError> + Send>>,
    pub format: Option<Box<dyn Fn(&Value, String) -> Result<String, ShellError>>>,
    pub eval: &'a Option<Box<dyn Fn(usize, &Value) -> Result<Value, ShellError> + Send>>,
}

pub fn report(
    values: &Value,
    options: Operation,
    tag: impl Into<Tag>,
) -> Result<Model, ShellError> {
    let tag = tag.into();

    let grouped = crate::utils::data::group(&values, &options.grouper, &tag)?;
    let splitted = crate::utils::data::split(&grouped, &options.splitter, &tag)?;

    let x = grouped
        .row_entries()
        .map(|(k, v)| k.clone())
        .collect::<Vec<_>>();

    let x = if options.format.is_some() {
        sort_columns(&x, &options.format)
    } else {
        sort_columns(&x, &None)
    }?;

    let mut y = splitted
        .row_entries()
        .map(|(k, v)| k.clone())
        .collect::<Vec<_>>();
    y.sort();

    let planes = Labels { x: x, y: y };
    let sorted = sort(&planes, &splitted, &tag)?;

    let evaluated = evaluate(
        &sorted,
        if options.eval.is_some() {
            options.eval
        } else {
            &None
        },
        &tag,
    )?;

    let group_labels = planes.grouping_total();
    let reduced = reduce(&evaluated, &tag)?;
    let max = max(&reduced, &tag)?.clone();
    let maxima = max.clone();
    let percents = percentages(&maxima, &reduced, &tag)?;

    /*let percentages = evaluate(
        &reduced,
        &Some(Box::new(move |_, row: &Value| {
            let hundred = UntaggedValue::decimal(100);
            let maxima = compute_values(Operator::Divide, &hundred, &maxima).unwrap();
            Ok(compute_values(Operator::Multiply, row, &maxima)
                .unwrap()
                .into_untagged_value())
                //Err(ShellError::labeled_error("err", "err", Tag::unknown()))
        })),
        &tag,
    )?;*/

    //println!("{:#?}", percentages);

    Ok(Model {
        labels: planes,
        ranges: (
            Range {
                start: UntaggedValue::int(0).into_untagged_value(),
                end: group_labels,
            },
            Range {
                start: UntaggedValue::int(0).into_untagged_value(),
                end: max,
            },
        ),
        data: reduced,
        percentages: percents,
    })
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
                y.push(
                    grouped
                        .table_entries()
                        .map(|x| x.clone())
                        .collect::<Vec<_>>(),
                );
            } else {
                let empty = UntaggedValue::table(&vec![]).into_value(&tag);
                y.push(empty.table_entries().map(|x| x.clone()).collect::<Vec<_>>());
            }
        }

        x.push(
            UntaggedValue::table(
                &y.iter()
                    .map(|x| x.clone())
                    .flatten()
                    .collect::<Vec<Value>>(),
            )
            .into_value(&tag),
        );
    }

    Ok(UntaggedValue::table(&x).into_value(&tag))
}

pub fn percentages(
    maxima: &Value,
    values: &Value,
    tag: impl Into<Tag>,
) -> Result<Value, ShellError> {
    let tag = tag.into();

    let mut x = vec![];
    for split in values.table_entries() {
        //let mut y = vec![];

        // for group in split.table_entries() {

        x.push(
            UntaggedValue::table(
                &split
                    .table_entries()
                    .map(|s| {
                        //for (idx, d) in subset.table_entries().enumerate() {
                        let hundred = UntaggedValue::decimal(100);
                        let maxima = compute_values(Operator::Divide, &hundred, &maxima).unwrap();

                        compute_values(Operator::Multiply, s, &maxima)
                            .unwrap()
                            .into_untagged_value()
                    })
                    .collect::<Vec<_>>(),
            )
            .into_value(&tag),
        );

        // }
        /*
        x.push(
            UntaggedValue::table(
                &y,
            )
            .into_value(&tag),
        );*/
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

        // for group in split.table_entries() {

        for subset in split.table_entries() {
            let mut set = vec![];
            for (idx, d) in subset.table_entries().enumerate() {
                if let Some(ref evaluator) = evaluator {
                    let value = evaluator(idx, d)?;

                    set.push(value);
                } else {
                    set.push(UntaggedValue::int(1).into_value(&tag));
                }
            }
            y.push(UntaggedValue::table(&set).into_value(&tag));
        }

        // }

        x.push(UntaggedValue::table(&y).into_value(&tag));
    }

    Ok(UntaggedValue::table(&x).into_value(&tag))
}

pub mod helpers {
    use super::{evaluate, sort, sort_columns, Labels, Operation};
    use super::{report, Model, Range};
    use indexmap::indexmap;
    use nu_errors::ShellError;
    use nu_protocol::{UntaggedValue, Value};
    use nu_source::{Tag, TaggedItem};
    use nu_value_ext::ValueExt;
    use num_bigint::BigInt;

    use indexmap::IndexMap;

    pub fn int(s: impl Into<BigInt>) -> Value {
        UntaggedValue::int(s).into_untagged_value()
    }

    pub fn string(input: impl Into<String>) -> Value {
        UntaggedValue::string(input.into()).into_untagged_value()
    }

    pub fn row(entries: IndexMap<String, Value>) -> Value {
        UntaggedValue::row(entries).into_untagged_value()
    }

    pub fn table(list: &[Value]) -> Value {
        UntaggedValue::table(list).into_untagged_value()
    }

    pub fn date(input: impl Into<String>) -> Value {
        let key = input.into().tagged_unknown();
        crate::data::value::Date::naive_from_str(key.borrow_tagged())
            .unwrap()
            .into_untagged_value()
    }

    pub fn committers() -> Vec<Value> {
        vec![
            row(indexmap! {
                   "date".into() => date("2019-07-23"),
                   "name".into() =>       string("AR"),
                "country".into() =>       string("EC"),
              "chickens".into() =>             int(10),
            }),
            row(indexmap! {
                   "date".into() => date("2019-07-23"),
                   "name".into() =>       string("JT"),
                "country".into() =>       string("NZ"),
               "chickens".into() =>             int(5),
            }),
            row(indexmap! {
                   "date".into() => date("2019-10-10"),
                   "name".into() =>       string("YK"),
                "country".into() =>       string("US"),
               "chickens".into() =>             int(6),
            }),
            row(indexmap! {
                   "date".into() => date("2019-09-24"),
                   "name".into() =>       string("AR"),
                "country".into() =>       string("EC"),
               "chickens".into() =>            int(20),
            }),
            row(indexmap! {
                   "date".into() => date("2019-10-10"),
                   "name".into() =>       string("JT"),
                "country".into() =>       string("NZ"),
               "chickens".into() =>            int(15),
            }),
            row(indexmap! {
                   "date".into() => date("2019-09-24"),
                   "name".into() =>       string("YK"),
                "country".into() =>       string("US"),
               "chickens".into() =>             int(4),
            }),
            row(indexmap! {
                   "date".into() => date("2019-10-10"),
                   "name".into() =>       string("AR"),
                "country".into() =>       string("EC"),
               "chickens".into() =>            int(30),
            }),
            row(indexmap! {
                   "date".into() => date("2019-09-24"),
                   "name".into() =>       string("JT"),
                "country".into() =>       string("NZ"),
              "chickens".into() =>             int(10),
            }),
            row(indexmap! {
                   "date".into() => date("2019-07-23"),
                   "name".into() =>       string("YK"),
                "country".into() =>       string("US"),
               "chickens".into() =>             int(2),
            }),
        ]
    }

    pub fn committers_grouped_by_date() -> Value {
        let sample = table(&committers());

        let grouper = Box::new(move |_, row: &Value| {
            let key = String::from("date").tagged_unknown();
            let group_key = row.get_data_by_key(key.borrow_spanned()).unwrap();

            group_key.format("%Y-%m-%d")
        });

        crate::utils::data::group(&sample, &Some(grouper), Tag::unknown()).unwrap()
    }

    pub fn datasets_sorted_by_date() -> Value {
        let key = String::from("date").tagged_unknown();

        crate::utils::data_processing::t_sort(
            Some(key),
            None,
            &committers_grouped_by_date(),
            Tag::unknown(),
        )
        .unwrap()
    }

    pub fn datasets_evaluated_by_default_one() -> Value {
        evaluate(&datasets_sorted_by_date(), &None, Tag::unknown()).unwrap()
    }

    pub fn date_formatter(
        fmt: &'static str,
    ) -> Box<dyn Fn(&Value, String) -> Result<String, ShellError>> {
        Box::new(move |date: &Value, _: String| date.format(&fmt))
    }

    #[test]
    fn builds_model() {
        let committers = table(&committers());

        let by_date = Box::new(move |_, row: &Value| {
            let key = String::from("date").tagged_unknown();
            let key = row.get_data_by_key(key.borrow_spanned()).unwrap();

            let callback = date_formatter("%Y-%m-%d");
            callback(&key, "nothing".to_string())
        });

        let by_country = Box::new(move |_, row: &Value| {
            let key = String::from("country").tagged_unknown();
            let key = row.get_data_by_key(key.borrow_spanned()).unwrap();

            nu_value_ext::as_string(&key)
        });

        /*let options = Operation {
            grouper: Some(by_date),
            splitter: Some(by_country),
            format: Some(date_formatter("%Y-%m-%d")),
            eval: Some(Box::new(move |_, value: &Value| {
                let chickens_key = String::from("chickens").tagged_unknown();

                Ok(value
                    .get_data_by_key(chickens_key.borrow_spanned())
                    .unwrap())
            })),
        };*/

        let options = Operation {
            grouper: Some(by_date),
            splitter: Some(by_country),
            format: Some(date_formatter("%Y-%m-%d")),
            eval: &Some(Box::new(move |_, value: &Value| {
                let chickens_key = String::from("chickens").tagged_unknown();

                Ok(value
                    .get_data_by_key(chickens_key.borrow_spanned())
                    .unwrap())
            })),
        };

        let summary_chickens_owned_by_committers_per_day =
            report(&committers, options, Tag::unknown()).unwrap();

        assert_eq!(
            summary_chickens_owned_by_committers_per_day.labels.x,
            vec![
                String::from("2019-07-23"),
                String::from("2019-09-24"),
                String::from("2019-10-10")
            ]
        );

        assert_eq!(
            summary_chickens_owned_by_committers_per_day.labels.y,
            vec![String::from("EC"), String::from("NZ"), String::from("US")]
        );

        assert_eq!(
            summary_chickens_owned_by_committers_per_day.ranges.0,
            Range {
                start: int(0),
                end: int(3)
            }
        );

        assert_eq!(
            summary_chickens_owned_by_committers_per_day.ranges.1,
            Range {
                start: int(0),
                end: int(60)
            }
        );

        assert_eq!(
            summary_chickens_owned_by_committers_per_day.data,
            table(&[
                table(&[int(10), int(30), int(60)]),
                table(&[int(5), int(15), int(30)]),
                table(&[int(2), int(6), int(12)]),
            ])
        );
    }
}
