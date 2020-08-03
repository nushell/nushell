use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue, Value};
use nu_source::Tagged;

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

    let column_grouper = column_name.clone();

    let results = crate::utils::data::report(
        &UntaggedValue::table(&values).into_value(&name),
        crate::utils::data::Operation {
            grouper: Some(Box::new(move |_, _| Ok(String::from("frequencies")))),
            splitter: Some(Box::new(move |_, row: &Value| {
                let key = &column_grouper;

                match row.get_data_by_key(key.borrow_spanned()) {
                    Some(key) => nu_value_ext::as_string(&key),
                    None => Err(ShellError::labeled_error(
                        "unknown column",
                        "unknown column",
                        key.tag(),
                    )),
                }
            })),
            format: None,
            eval: &None,
        },
        &name,
    )?;

    let labels = results.labels.y.clone();
    let column_names_supplied: Vec<_> = rest.iter().map(|f| f.item.clone()).collect();

    let frequency_column_name = if column_names_supplied.is_empty() {
        "frequency".to_string()
    } else {
        column_names_supplied[0].clone()
    };

    let column = (*column_name).clone();
    let mut idx = 0;

    Ok(futures::stream::iter(
        results
            .percentages
            .table_entries()
            .map(move |value| {
                let values = value.table_entries().cloned().collect::<Vec<_>>();
                let count = values.len();

                (count, values[count - 1].clone())
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(move |(count, value)| {
                let mut fact = TaggedDictBuilder::new(&name);
                let column_value = labels
                    .get(idx)
                    .ok_or_else(|| {
                        ShellError::labeled_error(
                            "Unable to load group labels",
                            "unabled to load group labels",
                            &name,
                        )
                    })?
                    .clone();

                fact.insert_value(&column, column_value);
                fact.insert_untagged("count", UntaggedValue::int(count));

                let string = std::iter::repeat("*")
                    .take(value.as_u64().map_err(|_| {
                        ShellError::labeled_error("expected a number", "expected a number", &name)
                    })? as usize)
                    .collect::<String>();

                fact.insert_untagged(&frequency_column_name, UntaggedValue::string(string));

                idx += 1;

                ReturnSuccess::value(fact.into_value())
            }),
    )
    .to_output_stream())
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
