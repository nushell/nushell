use crate::commands::evaluate_by::evaluate;
use crate::commands::group_by::group;
use crate::commands::map_max_by::map_max;
use crate::commands::reduce_by::reduce;
use crate::commands::t_sort_by::columns_sorted;
use crate::commands::t_sort_by::t_sort;
use crate::commands::WholeStreamCommand;
use crate::data::{value, TaggedDictBuilder};
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;
use num_traits::cast::ToPrimitive;

pub struct Histogram;

#[derive(Deserialize)]
pub struct HistogramArgs {
    column_name: Tagged<String>,
    rest: Vec<Tagged<String>>,
}

impl WholeStreamCommand for Histogram {
    fn name(&self) -> &str {
        "histogram"
    }

    fn signature(&self) -> Signature {
        Signature::build("histogram")
            .required(
                "column_name",
                SyntaxShape::String,
                "the name of the column to graph by",
            )
            .rest(
                SyntaxShape::Member,
                "column name to give the histogram's frequency column",
            )
    }

    fn usage(&self) -> &str {
        "Creates a new table with a histogram based on the column name passed in."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, histogram)?.run()
    }
}

pub fn histogram(
    HistogramArgs { column_name, rest }: HistogramArgs,
    RunnableContext { input, name, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let stream = async_stream! {
        let values: Vec<Value> = input.values.collect().await;

        let Tagged { item: group_by, .. } = column_name.clone();

        let groups = group(&column_name, values, &name)?;
        let group_labels = columns_sorted(Some(group_by.clone()), &groups, &name);
        let sorted = t_sort(Some(group_by.clone()), None, &groups, &name)?;
        let evaled = evaluate(&sorted, None, &name)?;
        let reduced = reduce(&evaled, None, &name)?;
        let maxima = map_max(&reduced, None, &name)?;
        let percents = percentages(&reduced, maxima, &name)?;

        match percents {
            Value {
                value: UntaggedValue::Table(datasets),
                ..
            } => {

                let mut idx = 0;

                let column_names_supplied: Vec<_> = rest.iter().map(|f| f.item.clone()).collect();

                let frequency_column_name = if column_names_supplied.is_empty() {
                    "frecuency".to_string()
                } else {
                    column_names_supplied[0].clone()
                };

                let column = (*column_name).clone();

                if let Value { value: UntaggedValue::Table(start), .. } = datasets.get(0).unwrap() {
                    for percentage in start.into_iter() {

                        let mut fact = TaggedDictBuilder::new(&name);
                        let value: Tagged<String> = group_labels.get(idx).unwrap().clone();
                        fact.insert_value(&column, value::string(value.item).into_value(value.tag));

                        if let Value { value: UntaggedValue::Primitive(Primitive::Int(ref num)), .. } = percentage.clone() {
                            let string = std::iter::repeat("*").take(num.to_i32().unwrap() as usize).collect::<String>();
                            fact.insert_untagged(&frequency_column_name, value::string(string));
                        }

                        idx = idx + 1;

                        yield ReturnSuccess::value(fact.into_value());
                    }
                }
            }
            _ => {}
        }
    };

    Ok(stream.to_output_stream())
}

fn percentages(values: &Value, max: Value, tag: impl Into<Tag>) -> Result<Value, ShellError> {
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
                        let data =
                                data.into_iter()
                                    .map(|d| match d {
                                        Value {
                                            value: UntaggedValue::Primitive(Primitive::Int(n)),
                                            ..
                                        } => {
                                            let max = match max {
                                                Value {
                                                    value:
                                                        UntaggedValue::Primitive(Primitive::Int(
                                                            ref maxima,
                                                        )),
                                                    ..
                                                } => maxima.to_i32().unwrap(),
                                                _ => 0,
                                            };

                                            let n = { n.to_i32().unwrap() * 100 / max };

                                            value::number(n).into_value(&tag)
                                        }
                                        _ => value::number(0).into_value(&tag),
                                    })
                                    .collect::<Vec<_>>();
                        UntaggedValue::Table(data).into_value(&tag)
                    }
                    _ => UntaggedValue::Table(vec![]).into_value(&tag),
                })
                .collect();

            UntaggedValue::Table(datasets).into_value(&tag)
        }
        other => other.clone(),
    };

    Ok(results)
}
