use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, Value};
use nu_source::Tagged;

pub struct GroupByDate;

#[derive(Deserialize)]
pub struct GroupByDateArgs {
    column_name: Option<Tagged<String>>,
    format: Option<Tagged<String>>,
}

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
        "Creates a new table with the data from the table rows grouped by the column given."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        group_by_date(args, registry)
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
    ByDate(Option<String>),
}

pub fn group_by_date(
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let name = args.call_info.name_tag.clone();
    let stream = async_stream! {
        let (GroupByDateArgs { column_name, format }, mut input) = args.process(&registry).await?;
        let values: Vec<Value> = input.collect().await;

        if values.is_empty() {
            yield Err(ShellError::labeled_error(
                    "Expected table from pipeline",
                    "requires a table input",
                    name
                ))
        } else {

            let grouper = if let Some(Tagged { item: fmt, tag }) = format {
                    Grouper::ByDate(Some(fmt))
                } else {
                    Grouper::ByDate(None)
                };

            match grouper {
                Grouper::ByDate(None) => {
                    match crate::utils::data::group(column_name, &values, Some(Box::new(|row: &Value| row.format("%Y-%b-%d"))), &name) {
                        Ok(grouped) => yield ReturnSuccess::value(grouped),
                        Err(err) => yield Err(err),
                    }
                }
                Grouper::ByDate(Some(fmt)) => {
                    match crate::utils::data::group(column_name, &values, Some(Box::new(move |row: &Value| {
                        row.format(&fmt)
                    })), &name) {
                        Ok(grouped) => yield ReturnSuccess::value(grouped),
                        Err(err) => yield Err(err),
                    }
                }
            }
        }
    };

    Ok(stream.to_output_stream())
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
