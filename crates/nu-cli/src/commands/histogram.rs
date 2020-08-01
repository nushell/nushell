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

#[async_trait]
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
                SyntaxShape::String,
                "column name to give the histogram's frequency column",
            )
    }

    fn usage(&self) -> &str {
        "Creates a new table with a histogram based on the column name passed in."
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        histogram(args, registry).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Get a histogram for the types of files",
                example: "ls | histogram type",
                result: None,
            },
            Example {
                description:
                    "Get a histogram for the types of files, with frequency column named count",
                example: "ls | histogram type count",
                result: None,
            },
            Example {
                description: "Get a histogram for a list of numbers",
                example: "echo [1 2 3 1 1 1 2 2 1 1] | wrap values | histogram values",
                result: None,
            },
        ]
    }
}

pub async fn histogram(
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let name = args.call_info.name_tag.clone();

    let (HistogramArgs { column_name, rest }, input) = args.process(&registry).await?;
    let values: Vec<Value> = input.collect().await;
    let values = UntaggedValue::table(&values).into_value(&name);

    let groups = group(&Some(column_name.clone()), &values, &name)?;
    let group_labels = columns_sorted(Some(column_name.clone()), &groups, &name);
    let sorted = t_sort(Some(column_name.clone()), None, &groups, &name)?;
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

            let count_column_name = "count".to_string();
            let count_shell_error = ShellError::labeled_error(
                "Unable to load group count",
                "unabled to load group count",
                &name,
            );
            let mut count_values: Vec<u64> = Vec::new();

            for table_entry in reduced.table_entries() {
                match table_entry {
                    Value {
                        value: UntaggedValue::Table(list),
                        ..
                    } => {
                        for i in list {
                            if let Ok(count) = i.value.clone().into_value(&name).as_u64() {
                                count_values.push(count);
                            } else {
                                return Err(count_shell_error);
                            }
                        }
                    }
                    _ => {
                        return Err(count_shell_error);
                    }
                }
            }

            if let Value {
                value: UntaggedValue::Table(start),
                ..
            } = datasets.get(0).ok_or_else(|| {
                ShellError::labeled_error(
                    "Unable to load dataset",
                    "unabled to load dataset",
                    &name,
                )
            })? {
                let start = start.clone();
                Ok(
                    futures::stream::iter(start.into_iter().map(move |percentage| {
                        let mut fact = TaggedDictBuilder::new(&name);
                        let value: Tagged<String> = group_labels
                            .get(idx)
                            .ok_or_else(|| {
                                ShellError::labeled_error(
                                    "Unable to load group labels",
                                    "unabled to load group labels",
                                    &name,
                                )
                            })?
                            .clone();
                        fact.insert_value(
                            &column,
                            UntaggedValue::string(value.item).into_value(value.tag),
                        );

                        fact.insert_untagged(
                            &count_column_name,
                            UntaggedValue::int(count_values[idx]),
                        );

                        if let Value {
                            value: UntaggedValue::Primitive(Primitive::Int(ref num)),
                            ref tag,
                        } = percentage
                        {
                            let string = std::iter::repeat("*")
                                .take(num.to_i32().ok_or_else(|| {
                                    ShellError::labeled_error(
                                        "Expected a number",
                                        "expected a number",
                                        tag,
                                    )
                                })? as usize)
                                .collect::<String>();
                            fact.insert_untagged(
                                &frequency_column_name,
                                UntaggedValue::string(string),
                            );
                        }

                        idx += 1;

                        ReturnSuccess::value(fact.into_value())
                    }))
                    .to_output_stream(),
                )
            } else {
                Ok(OutputStream::empty())
            }
        }
        _ => Ok(OutputStream::empty()),
    }
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

#[cfg(test)]
mod tests {
    use super::Histogram;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Histogram {})
    }
}
