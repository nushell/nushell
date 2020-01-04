use crate::commands::group_by::group;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use crate::utils::data_processing::{columns_sorted, evaluate, map_max, reduce, t_sort};
use nu_errors::ShellError;
use nu_protocol::{
    Primitive, ReturnSuccess, Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue, Value,
};
use nu_source::Tagged;
use num_traits::{ToPrimitive, Zero};

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
                    "frequency".to_string()
                } else {
                    column_names_supplied[0].clone()
                };

                let column = (*column_name).clone();

                if let Value { value: UntaggedValue::Table(start), .. } = datasets.get(0).ok_or_else(|| ShellError::labeled_error("Unable to load dataset", "unabled to load dataset", &name))? {
                    for percentage in start.iter() {

                        let mut fact = TaggedDictBuilder::new(&name);
                        let value: Tagged<String> = group_labels.get(idx).ok_or_else(|| ShellError::labeled_error("Unable to load group labels", "unabled to load group labels", &name))?.clone();
                        fact.insert_value(&column, UntaggedValue::string(value.item).into_value(value.tag));

                        if let Value { value: UntaggedValue::Primitive(Primitive::Int(ref num)), ref tag } = percentage.clone() {
                            let string = std::iter::repeat("*").take(num.to_i32().ok_or_else(|| ShellError::labeled_error("Expected a number", "expected a number", tag))? as usize).collect::<String>();
                            fact.insert_untagged(&frequency_column_name, UntaggedValue::string(string));
                        }

                        idx += 1;

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
                .iter()
                .map(|subsets| match subsets {
                    Value {
                        value: UntaggedValue::Table(data),
                        ..
                    } => {
                        let data = data
                            .iter()
                            .map(|d| match d {
                                Value {
                                    value: UntaggedValue::Primitive(Primitive::Int(n)),
                                    ..
                                } => {
                                    let max = match &max {
                                        Value {
                                            value: UntaggedValue::Primitive(Primitive::Int(maxima)),
                                            ..
                                        } => maxima.clone(),
                                        _ => Zero::zero(),
                                    };

                                    let n = (n * 100) / max;

                                    UntaggedValue::int(n).into_value(&tag)
                                }
                                _ => UntaggedValue::int(0).into_value(&tag),
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
