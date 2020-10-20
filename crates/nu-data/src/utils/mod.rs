pub mod group;
pub mod split;

mod internal;

pub use crate::utils::group::group;
pub use crate::utils::split::split;

pub use crate::utils::internal::Reduction;
use crate::utils::internal::*;

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
    pub format: &'a Option<Box<dyn Fn(&Value, String) -> Result<String, ShellError>>>,
    pub eval: &'a Option<Box<dyn Fn(usize, &Value) -> Result<Value, ShellError> + Send>>,
    pub reduction: &'a Reduction,
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

    let x = sort_columns(&x, &options.format)?;

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

    let reduced = reduce(&evaluated, options.reduction, &tag)?;

    let maxima = max(&reduced, &tag)?;

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
                end: maxima,
            },
        ),
        data: reduced,
        percentages: percents,
    })
}

pub mod helpers {
    use nu_errors::ShellError;
    use nu_protocol::{row, Value};
    use nu_source::{Tag, TaggedItem};
    use nu_test_support::value::{date, int, string, table};
    use nu_value_ext::ValueExt;

    pub fn committers() -> Vec<Value> {
        vec![
            row! {
                   "date".into() => date("2019-07-23"),
                   "name".into() =>       string("AR"),
                "country".into() =>       string("EC"),
              "chickens".into() =>             int(10)
            },
            row! {
                   "date".into() => date("2019-07-23"),
                   "name".into() =>       string("JT"),
                "country".into() =>       string("NZ"),
               "chickens".into() =>             int(5)
            },
            row! {
                   "date".into() => date("2019-10-10"),
                   "name".into() =>       string("YK"),
                "country".into() =>       string("US"),
               "chickens".into() =>             int(6)
            },
            row! {
                   "date".into() => date("2019-09-24"),
                   "name".into() =>       string("AR"),
                "country".into() =>       string("EC"),
               "chickens".into() =>            int(20)
            },
            row! {
                   "date".into() => date("2019-10-10"),
                   "name".into() =>       string("JT"),
                "country".into() =>       string("NZ"),
               "chickens".into() =>            int(15)
            },
            row! {
                   "date".into() => date("2019-09-24"),
                   "name".into() =>       string("YK"),
                "country".into() =>       string("US"),
               "chickens".into() =>             int(4)
            },
            row! {
                   "date".into() => date("2019-10-10"),
                   "name".into() =>       string("AR"),
                "country".into() =>       string("EC"),
               "chickens".into() =>            int(30)
            },
            row! {
                   "date".into() => date("2019-09-24"),
                   "name".into() =>       string("JT"),
                "country".into() =>       string("NZ"),
              "chickens".into() =>             int(10)
            },
            row! {
                   "date".into() => date("2019-07-23"),
                   "name".into() =>       string("YK"),
                "country".into() =>       string("US"),
               "chickens".into() =>             int(2)
            },
        ]
    }

    pub fn committers_grouped_by_date() -> Value {
        let sample = table(&committers());

        let grouper = Box::new(move |_, row: &Value| {
            let key = String::from("date").tagged_unknown();
            let group_key = row
                .get_data_by_key(key.borrow_spanned())
                .expect("get key failed");

            group_key.format("%Y-%m-%d")
        });

        crate::utils::group(&sample, &Some(grouper), Tag::unknown())
            .expect("failed to create group")
    }

    pub fn date_formatter(
        fmt: String,
    ) -> Box<dyn Fn(&Value, String) -> Result<String, ShellError>> {
        Box::new(move |date: &Value, _: String| {
            let fmt = fmt.clone();
            date.format(&fmt)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::helpers::{committers, date_formatter};
    use super::{report, Labels, Model, Operation, Range, Reduction};
    use nu_errors::ShellError;
    use nu_protocol::Value;
    use nu_source::{Tag, TaggedItem};
    use nu_test_support::value::{decimal_from_float, int, table};
    use nu_value_ext::ValueExt;

    pub fn assert_without_checking_percentages(report_a: Model, report_b: Model) {
        assert_eq!(report_a.labels.x, report_b.labels.x);
        assert_eq!(report_a.labels.y, report_b.labels.y);
        assert_eq!(report_a.ranges, report_b.ranges);
        assert_eq!(report_a.data, report_b.data);
    }

    #[test]
    fn prepares_report_using_counting_value() {
        let committers = table(&committers());

        let by_date = Box::new(move |_, row: &Value| {
            let key = String::from("date").tagged_unknown();
            let key = row.get_data_by_key(key.borrow_spanned()).unwrap();

            let callback = date_formatter("%Y-%m-%d".to_string());
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
            format: &None,
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
            reduction: &Reduction::Count
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
                        end: int(30),
                    },
                ),
                data: table(&[
                    table(&[int(10), int(20), int(30)]),
                    table(&[int(5), int(10), int(15)]),
                    table(&[int(2), int(4), int(6)]),
                ]),
                percentages: table(&[
                    table(&[
                        decimal_from_float(33.33),
                        decimal_from_float(66.66),
                        decimal_from_float(99.99),
                    ]),
                    table(&[
                        decimal_from_float(16.66),
                        decimal_from_float(33.33),
                        decimal_from_float(49.99),
                    ]),
                    table(&[
                        decimal_from_float(6.66),
                        decimal_from_float(13.33),
                        decimal_from_float(19.99),
                    ]),
                ]),
            },
        );
    }
}
