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

impl WholeStreamCommand for TSortBy {
    fn name(&self) -> &str {
        "t-sort-by"
    }

    fn signature(&self) -> Signature {
        Signature::build("t-sort-by")
            .switch("show-columns", "Displays the column names sorted")
            .named(
                "group_by",
                SyntaxShape::String,
                "the name of the column to group by",
            )
            .named(
                "split_by",
                SyntaxShape::String,
                "the name of the column within the grouped by table to split by",
            )
    }

    fn usage(&self) -> &str {
        "Sort by the given columns."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, t_sort_by)?.run()
    }
}

fn t_sort_by(
    TSortByArgs {
        show_columns,
        group_by,
        ..
    }: TSortByArgs,
    RunnableContext { input, name, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    Ok(OutputStream::new(async_stream! {
        let values: Vec<Value> = input.values.collect().await;

        let column_grouped_by_name = if let Some(grouped_by) = group_by {
            Some(grouped_by.item().clone())
        } else {
            None
        };

        if show_columns {
            for label in columns_sorted(column_grouped_by_name, &values[0], &name).into_iter() {
                 yield ReturnSuccess::value(UntaggedValue::string(label.item).into_value(label.tag));
            }
        } else {
            match t_sort(column_grouped_by_name, None, &values[0], name) {
                Ok(sorted) => yield ReturnSuccess::value(sorted),
                Err(err) => yield Err(err)
            }
        }
    }))
}
