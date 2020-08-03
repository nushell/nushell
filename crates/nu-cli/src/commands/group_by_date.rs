use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;

pub struct GroupByDate;

#[derive(Deserialize)]
pub struct GroupByDateArgs {
    column_name: Option<Tagged<String>>,
    format: Option<Tagged<String>>,
}

#[async_trait]
impl WholeStreamCommand for GroupByDate {
    fn name(&self) -> &str {
        "group-by date"
    }

    fn signature(&self) -> Signature {
        Signature::build("group-by date")
            .optional(
                "column_name",
                SyntaxShape::String,
                "the name of the column to group by",
            )
            .named(
                "format",
                SyntaxShape::String,
                "Specify date and time formatting",
                Some('f'),
            )
    }

    fn usage(&self) -> &str {
        "creates a table grouped by date."
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        group_by_date(args, registry).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Group files by type",
            example: "ls | group-by date --format '%d/%m/%Y'",
            result: None,
        }]
    }
}

enum Grouper {
    ByDate(Option<Tagged<String>>),
}

enum GroupByColumn {
    Name(Option<Tagged<String>>),
}

pub async fn group_by_date(
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let name = args.call_info.name_tag.clone();
    let (
        GroupByDateArgs {
            column_name,
            format,
        },
        input,
    ) = args.process(&registry).await?;
    let values: Vec<Value> = input.collect().await;

    if values.is_empty() {
        Err(ShellError::labeled_error(
            "Expected table from pipeline",
            "requires a table input",
            name,
        ))
    } else {
        let values = UntaggedValue::table(&values).into_value(&name);

        let grouper_column = if let Some(column_name) = column_name {
            GroupByColumn::Name(Some(column_name))
        } else {
            GroupByColumn::Name(None)
        };

        let grouper_date = if let Some(date_format) = format {
            Grouper::ByDate(Some(date_format))
        } else {
            Grouper::ByDate(None)
        };

        let value_result = match (grouper_date, grouper_column) {
            (Grouper::ByDate(None), GroupByColumn::Name(None)) => {
                let block = Box::new(move |_, row: &Value| row.format("%Y-%m-%d"));

                crate::utils::data::group(&values, &Some(block), &name)
            }
            (Grouper::ByDate(None), GroupByColumn::Name(Some(column_name))) => {
                let block = Box::new(move |_, row: &Value| {
                    let group_key = row
                        .get_data_by_key(column_name.borrow_spanned())
                        .ok_or_else(|| suggestions(column_name.borrow_tagged(), &row));

                    group_key?.format("%Y-%m-%d")
                });

                crate::utils::data::group(&values, &Some(block), &name)
            }
            (Grouper::ByDate(Some(fmt)), GroupByColumn::Name(None)) => {
                let block = Box::new(move |_, row: &Value| row.format(&fmt));

                crate::utils::data::group(&values, &Some(block), &name)
            }
            (Grouper::ByDate(Some(fmt)), GroupByColumn::Name(Some(column_name))) => {
                let block = Box::new(move |_, row: &Value| {
                    let group_key = row
                        .get_data_by_key(column_name.borrow_spanned())
                        .ok_or_else(|| suggestions(column_name.borrow_tagged(), &row));

                    group_key?.format(&fmt)
                });

                crate::utils::data::group(&values, &Some(block), &name)
            }
        };

        Ok(OutputStream::one(ReturnSuccess::value(value_result?)))
    }
}

pub fn suggestions(tried: Tagged<&str>, for_value: &Value) -> ShellError {
    let possibilities = for_value.data_descriptors();

    let mut possible_matches: Vec<_> = possibilities
        .iter()
        .map(|x| (natural::distance::levenshtein_distance(x, &tried), x))
        .collect();

    possible_matches.sort();

    if !possible_matches.is_empty() {
        ShellError::labeled_error(
            "Unknown column",
            format!("did you mean '{}'?", possible_matches[0].1),
            tried.tag(),
        )
    } else {
        ShellError::labeled_error(
            "Unknown column",
            "row does not contain this column",
            tried.tag(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::GroupByDate;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(GroupByDate {})
    }
}
