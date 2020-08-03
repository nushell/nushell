pub mod group;
pub mod split;

mod internal;

pub use crate::utils::data::group::group;
pub use crate::utils::data::split::split;

use crate::utils::data::internal::*;

use derive_new::new;
use getset::Getters;
use nu_errors::ShellError;
use nu_protocol::{UntaggedValue, Value};
use nu_source::Tag;

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Getters, Clone, new)]
pub struct Model {
    pub labels: Labels,
    pub ranges: (Range, Range),

    pub data: Value,
    pub percentages: Value,
}

#[allow(clippy::type_complexity)]
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

    let grouped = group(&values, &options.grouper, &tag)?;
    let splitted = split(&grouped, &options.splitter, &tag)?;

    let x = grouped
        .row_entries()
        .map(|(key, _)| key.clone())
        .collect::<Vec<_>>();

    let x = if options.format.is_some() {
        sort_columns(&x, &options.format)
    } else {
        sort_columns(&x, &None)
    }?;

    let mut y = splitted
        .row_entries()
        .map(|(key, _)| key.clone())
        .collect::<Vec<_>>();
    y.sort();

    let planes = Labels { x, y };
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

#[cfg(test)]
pub mod helpers {
    use super::{report, Labels, Model, Operation, Range};
    use bigdecimal::BigDecimal;
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

    pub fn decimal(f: impl Into<BigDecimal>) -> Value {
        UntaggedValue::decimal(f.into()).into_untagged_value()
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

    pub fn date_formatter(
        fmt: &'static str,
    ) -> Box<dyn Fn(&Value, String) -> Result<String, ShellError>> {
        Box::new(move |date: &Value, _: String| date.format(&fmt))
    }

    fn assert_without_checking_percentages(report_a: Model, report_b: Model) {
        assert_eq!(report_a.labels.x, report_b.labels.x);
        assert_eq!(report_a.labels.y, report_b.labels.y);
        assert_eq!(report_a.ranges, report_b.ranges);
        assert_eq!(report_a.data, report_b.data);
    }

    #[test]
    fn prepares_report_using_accumulating_value() {
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

        let options = Operation {
            grouper: Some(by_date),
            splitter: Some(by_country),
            format: Some(date_formatter("%Y-%m-%d")),
            eval: /* value to be used for accumulation */ &Some(Box::new(move |_, value: &Value| {
                let chickens_key = String::from("chickens").tagged_unknown();

                value
                    .get_data_by_key(chickens_key.borrow_spanned())
                    .ok_or_else(|| {
                        ShellError::labeled_error(
                            "unknown column",
                            "unknown column",
                            chickens_key.span(),
                        )
                    })
            })),
        };

        assert_without_checking_percentages(
            report(&committers, options, Tag::unknown()).unwrap(),
            Model {
                labels: Labels {
                    x: vec![
                        String::from("2019-07-23"),
                        String::from("2019-09-24"),
                        String::from("2019-10-10"),
                    ],
                    y: vec![String::from("EC"), String::from("NZ"), String::from("US")],
                },
                ranges: (
                    Range {
                        start: int(0),
                        end: int(3),
                    },
                    Range {
                        start: int(0),
                        end: int(60),
                    },
                ),
                data: table(&[
                    table(&[int(10), int(30), int(60)]),
                    table(&[int(5), int(15), int(30)]),
                    table(&[int(2), int(6), int(12)]),
                ]),
                percentages: table(&[
                    table(&[decimal(16.66), decimal(50), decimal(100)]),
                    table(&[decimal(8.33), decimal(25), decimal(50)]),
                    table(&[decimal(3.33), decimal(10), decimal(20)]),
                ]),
            },
        );
    }
}
