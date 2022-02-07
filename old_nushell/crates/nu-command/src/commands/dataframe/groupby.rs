use crate::{commands::dataframe::utils::parse_polars_error, prelude::*};
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    dataframe::{FrameStruct, NuDataFrame, NuGroupBy},
    Signature, SyntaxShape, UntaggedValue, Value,
};

use super::utils::convert_columns;

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "dataframe group-by"
    }

    fn usage(&self) -> &str {
        "[DataFrame] Creates a groupby object that can be used for other aggregations"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe group-by").rest("rest", SyntaxShape::Any, "groupby columns")
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Grouping by column a",
            example: "[[a b]; [one 1] [one 2]] | dataframe to-df | dataframe group-by a",
            result: None,
        }]
    }
}

fn command(mut args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();

    // Extracting the names of the columns to perform the groupby
    let by_columns: Vec<Value> = args.rest(0)?;
    let (columns_string, col_span) = convert_columns(&by_columns, &tag)?;

    let (df, _) = NuDataFrame::try_from_stream(&mut args.input, &tag.span)?;

    // This is the expensive part of the groupby; to create the
    // groups that will be used for grouping the data in the
    // dataframe. Once it has been done these values can be stored
    // in a NuGroupBy
    let groupby = df
        .as_ref()
        .groupby(&columns_string)
        .map_err(|e| parse_polars_error::<&str>(&e, &col_span, None))?;

    let groups = groupby.get_groups().to_vec();
    let groupby = Value {
        tag,
        value: UntaggedValue::FrameStruct(FrameStruct::GroupBy(NuGroupBy::new(
            NuDataFrame::new(df.as_ref().clone()),
            columns_string,
            groups,
        ))),
    };

    Ok(OutputStream::one(groupby))
}
