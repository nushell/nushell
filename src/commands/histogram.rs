use crate::commands::evaluate_by::evaluate;
use crate::commands::group_by::group;
use crate::commands::map_max_by::map_max;
use crate::commands::reduce_by::reduce;
use crate::commands::t_sort_by::columns_sorted;
use crate::commands::t_sort_by::t_sort;
use crate::commands::WholeStreamCommand;
use crate::data::TaggedDictBuilder;
use crate::errors::ShellError;
use crate::prelude::*;
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
        let values: Vec<Tagged<Value>> = input.values.collect().await;

        let Tagged { item: group_by, .. } = column_name.clone();

        let groups = group(&column_name, values, &name)?;
        let group_labels = columns_sorted(Some(group_by.clone()), &groups, &name);
        let sorted = t_sort(Some(group_by.clone()), None, &groups, &name)?;
        let evaled = evaluate(&sorted, None, &name)?;
        let reduced = reduce(&evaled, None, &name)?;
        let maxima = map_max(&reduced, None, &name)?;
        let percents = percentages(&reduced, maxima, &name)?;

        match percents {
            Tagged {
                item: Value::Table(datasets),
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

                if let Tagged { item: Value::Table(start), .. } = datasets.get(0).unwrap() {
                    for percentage in start.into_iter() {

                        let mut fact = TaggedDictBuilder::new(&name);
                        let value: Tagged<String> = group_labels.get(idx).unwrap().clone();
                        fact.insert_tagged(&column, Value::string(value.item).tagged(value.tag));

                        if let Tagged { item: Value::Primitive(Primitive::Int(ref num)), .. } = percentage.clone() {
                            fact.insert(&frequency_column_name, std::iter::repeat("*").take(num.to_i32().unwrap() as usize).collect::<String>());
                        }

                        idx = idx + 1;

                        yield ReturnSuccess::value(fact.into_tagged_value());
                    }
                }
            }
            _ => {}
        }
    };

    Ok(stream.to_output_stream())
}

fn percentages(
    values: &Tagged<Value>,
    max: Tagged<Value>,
    tag: impl Into<Tag>,
) -> Result<Tagged<Value>, ShellError> {
    let tag = tag.into();

    let results: Tagged<Value> = match values {
        Tagged {
            item: Value::Table(datasets),
            ..
        } => {
            let datasets: Vec<_> = datasets
                .into_iter()
                .map(|subsets| match subsets {
                    Tagged {
                        item: Value::Table(data),
                        ..
                    } => {
                        let data = data
                            .into_iter()
                            .map(|d| match d {
                                Tagged {
                                    item: Value::Primitive(Primitive::Int(n)),
                                    ..
                                } => {
                                    let max = match max {
                                        Tagged {
                                            item: Value::Primitive(Primitive::Int(ref maxima)),
                                            ..
                                        } => maxima.to_i32().unwrap(),
                                        _ => 0,
                                    };

                                    let n = { n.to_i32().unwrap() * 100 / max };

                                    Value::number(n).tagged(&tag)
                                }
                                _ => Value::number(0).tagged(&tag),
                            })
                            .collect::<Vec<_>>();
                        Value::Table(data).tagged(&tag)
                    }
                    _ => Value::Table(vec![]).tagged(&tag),
                })
                .collect();

            Value::Table(datasets).tagged(&tag)
        }
        other => other.clone(),
    };

    Ok(results)
}
