use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;
use num_traits::cast::ToPrimitive;

pub struct ReduceBy;

#[derive(Deserialize)]
pub struct ReduceByArgs {
    reduce_with: Option<Tagged<String>>,
}

impl WholeStreamCommand for ReduceBy {
    fn name(&self) -> &str {
        "reduce-by"
    }

    fn signature(&self) -> Signature {
        Signature::build("reduce-by").named(
            "reduce_with",
            SyntaxShape::String,
            "the command to reduce by with",
        )
    }

    fn usage(&self) -> &str {
        "Creates a new table with the data from the tables rows reduced by the command given."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, reduce_by)?.run()
    }
}

pub fn reduce_by(
    ReduceByArgs { reduce_with }: ReduceByArgs,
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

            let reduce_with = if let Some(reducer) = reduce_with {
                Some(reducer.item().clone())
            } else {
                None
            };

            match reduce(&values[0], reduce_with, name) {
                Ok(reduced) => yield ReturnSuccess::value(reduced),
                Err(err) => yield Err(err)
            }
        }
    };

    Ok(stream.to_output_stream())
}

fn sum(data: Vec<Value>) -> i32 {
    data.into_iter().fold(0, |acc, value| match value {
        Value {
            value: UntaggedValue::Primitive(Primitive::Int(n)),
            ..
        } => acc + n.to_i32().unwrap(),
        _ => acc,
    })
}

fn formula(
    acc_begin: i32,
    calculator: Box<dyn Fn(Vec<Value>) -> i32 + 'static>,
) -> Box<dyn Fn(i32, Vec<Value>) -> i32 + 'static> {
    Box::new(move |acc, datax| -> i32 {
        let result = acc * acc_begin;
        result + calculator(datax)
    })
}

fn reducer_for(command: Reduce) -> Box<dyn Fn(i32, Vec<Value>) -> i32 + 'static> {
    match command {
        Reduce::Sum | Reduce::Default => Box::new(formula(0, Box::new(sum))),
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
                .into_iter()
                .map(|subsets| {
                    let mut acc = 0;
                    match subsets {
                        Value {
                            value: UntaggedValue::Table(data),
                            ..
                        } => {
                            let data = data
                                .into_iter()
                                .map(|d| {
                                    if let Value {
                                        value: UntaggedValue::Table(x),
                                        ..
                                    } = d
                                    {
                                        acc = reduce_with(acc, x.clone());
                                        value::number(acc).into_value(&tag)
                                    } else {
                                        value::number(0).into_value(&tag)
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

#[cfg(test)]
mod tests {

    use crate::commands::evaluate_by::evaluate;
    use crate::commands::group_by::group;
    use crate::commands::reduce_by::{reduce, reducer_for, Reduce};
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

    fn table(list: &Vec<Value>) -> Value {
        value::table(list).into_untagged_value()
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

    fn nu_releases_evaluated_by_default_one() -> Value {
        evaluate(&nu_releases_sorted_by_date(), None, Tag::unknown()).unwrap()
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
        ]
    }

    #[test]
    fn reducer_computes_given_a_sum_command() {
        let subject = vec![int(1), int(1), int(1)];

        let action = reducer_for(Reduce::Sum);

        assert_eq!(action(0, subject), 3);
    }

    #[test]
    fn reducer_computes() {
        assert_eq!(
            reduce(
                &nu_releases_evaluated_by_default_one(),
                Some(String::from("sum")),
                Tag::unknown()
            ),
            Ok(table(&vec![table(&vec![int(3), int(3), int(3)])]))
        );
    }
}
