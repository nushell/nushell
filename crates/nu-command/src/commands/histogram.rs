use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    ColumnPath, ReturnSuccess, Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue, Value,
};
use nu_source::Tagged;

pub struct Histogram;

#[async_trait]
impl WholeStreamCommand for Histogram {
    fn name(&self) -> &str {
        "histogram"
    }

    fn signature(&self) -> Signature {
        Signature::build("histogram")
            .named(
                "use",
                SyntaxShape::ColumnPath,
                "Use data at the column path given as valuator",
                None,
            )
            .rest(
                SyntaxShape::ColumnPath,
                "column name to give the histogram's frequency column",
            )
    }

    fn usage(&self) -> &str {
        "Creates a new table with a histogram based on the column name passed in."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        histogram(args).await
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
                    "Get a histogram for the types of files, with frequency column named percentage",
                example: "ls | histogram type percentage",
                result: None,
            },
            Example {
                description: "Get a histogram for a list of numbers",
                example: "echo [1 2 3 1 1 1 2 2 1 1] | histogram",
                result: None,
            },
        ]
    }
}

pub async fn histogram(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let name = args.call_info.name_tag.clone();
    let (input, args) = args.evaluate_once().await?.parts();

    let values: Vec<Value> = input.collect().await;

    let mut columns = args
        .positional_iter()
        .map(|c| c.as_column_path())
        .filter_map(Result::ok)
        .collect::<Vec<_>>();

    let evaluate_with = if let Some(path) = args.get("use") {
        Some(evaluator(path.as_column_path()?.item))
    } else {
        None
    };

    let column_grouper = if !columns.is_empty() {
        match columns.remove(0).split_last() {
            Some((key, _)) => Some(key.as_string().tagged(&name)),
            None => None,
        }
    } else {
        None
    };

    let frequency_column_name = if columns.is_empty() {
        "frequency".to_string()
    } else if let Some((key, _)) = columns[0].split_last() {
        key.as_string()
    } else {
        "frequency".to_string()
    };

    let column = if let Some(ref column) = column_grouper {
        column.clone()
    } else {
        "value".to_string().tagged(&name)
    };

    let results = nu_data::utils::report(
        &UntaggedValue::table(&values).into_value(&name),
        nu_data::utils::Operation {
            grouper: Some(Box::new(move |_, _| Ok(String::from("frequencies")))),
            splitter: Some(splitter(column_grouper)),
            format: &None,
            eval: &evaluate_with,
            reduction: &nu_data::utils::Reduction::Count,
        },
        &name,
    )?;

    let labels = results.labels.y.clone();
    let mut idx = 0;

    Ok(futures::stream::iter(
        results
            .data
            .table_entries()
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .zip(
                results
                    .percentages
                    .table_entries()
                    .cloned()
                    .collect::<Vec<_>>()
                    .into_iter(),
            )
            .map(move |(counts, percentages)| {
                let percentage = percentages
                    .table_entries()
                    .cloned()
                    .last()
                    .unwrap_or_else(|| {
                        UntaggedValue::decimal_from_float(0.0, name.span).into_value(&name)
                    });
                let value = counts
                    .table_entries()
                    .cloned()
                    .last()
                    .unwrap_or_else(|| UntaggedValue::int(0).into_value(&name));

                let mut fact = TaggedDictBuilder::new(&name);
                let column_value = labels
                    .get(idx)
                    .ok_or_else(|| {
                        ShellError::labeled_error(
                            "Unable to load group labels",
                            "unable to load group labels",
                            &name,
                        )
                    })?
                    .clone();

                fact.insert_value(&column.item, column_value);
                fact.insert_untagged("count", value);

                let fmt_percentage = format!(
                    "{}%",
                    // Some(2) < the number of digits
                    // true < group the digits
                    crate::commands::str_::from::action(&percentage, &name, Some(2), true)?
                        .as_string()?
                );
                fact.insert_untagged("percentage", UntaggedValue::string(fmt_percentage));

                let string = std::iter::repeat("*")
                    .take(percentage.as_u64().map_err(|_| {
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

fn evaluator(by: ColumnPath) -> Box<dyn Fn(usize, &Value) -> Result<Value, ShellError> + Send> {
    Box::new(move |_: usize, value: &Value| {
        let path = by.clone();

        let eval = nu_value_ext::get_data_by_column_path(value, &path, move |_, _, error| error);

        match eval {
            Ok(with_value) => Ok(with_value),
            Err(reason) => Err(reason),
        }
    })
}

fn splitter(
    by: Option<Tagged<String>>,
) -> Box<dyn Fn(usize, &Value) -> Result<String, ShellError> + Send> {
    match by {
        Some(column) => Box::new(move |_, row: &Value| {
            let key = &column;

            match row.get_data_by_key(key.borrow_spanned()) {
                Some(key) => nu_value_ext::as_string(&key),
                None => Err(ShellError::labeled_error(
                    "unknown column",
                    "unknown column",
                    key.tag(),
                )),
            }
        }),
        None => Box::new(move |_, row: &Value| nu_value_ext::as_string(&row)),
    }
}

#[cfg(test)]
mod tests {
    use super::Histogram;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Histogram {})
    }
}
