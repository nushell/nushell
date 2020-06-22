use crate::commands::WholeStreamCommand;
use crate::data::TaggedListBuilder;
use crate::prelude::*;
use crate::utils::data_processing::{columns_sorted, t_sort};
use chrono::{DateTime, NaiveDate, Utc};
use nu_errors::ShellError;
use nu_protocol::{
    Primitive, ReturnSuccess, Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue, Value,
};
use nu_source::Tagged;
use nu_value_ext::get_data_by_key;

pub struct TSortBy;

#[derive(Deserialize)]
pub struct TSortByArgs {
    #[serde(rename(deserialize = "show-columns"))]
    show_columns: bool,
    group_by: Option<Tagged<String>>,
    #[allow(unused)]
    split_by: Option<String>,
}

#[async_trait]
impl WholeStreamCommand for TSortBy {
    fn name(&self) -> &str {
        "t-sort-by"
    }

    fn signature(&self) -> Signature {
        Signature::build("t-sort-by")
            .switch(
                "show-columns",
                "Displays the column names sorted",
                Some('c'),
            )
            .named(
                "group_by",
                SyntaxShape::String,
                "the name of the column to group by",
                Some('g'),
            )
            .named(
                "split_by",
                SyntaxShape::String,
                "the name of the column within the grouped by table to split by",
                Some('s'),
            )
    }

    fn usage(&self) -> &str {
        "Sort by the given columns."
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        t_sort_by(args, registry).await
    }
}

async fn t_sort_by(
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let name = args.call_info.name_tag.clone();
    let (
        TSortByArgs {
            show_columns,
            group_by,
            ..
        },
        mut input,
    ) = args.process(&registry).await?;
    let values: Vec<Value> = input.collect().await;

    let column_grouped_by_name = if let Some(grouped_by) = group_by {
        Some(grouped_by)
    } else {
        None
    };

    if show_columns {
        Ok(futures::stream::iter(
            columns_sorted(column_grouped_by_name, &values[0], &name)
                .into_iter()
                .map(move |label| {
                    ReturnSuccess::value(UntaggedValue::string(label.item).into_value(label.tag))
                }),
        )
        .to_output_stream())
    } else {
        match t_sort(column_grouped_by_name, None, &values[0], name) {
            Ok(sorted) => Ok(OutputStream::one(ReturnSuccess::value(sorted))),
            Err(err) => Ok(OutputStream::one(Err(err))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TSortBy;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(TSortBy {})
    }
}
