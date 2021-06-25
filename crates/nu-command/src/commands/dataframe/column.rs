use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    dataframe::{NuDataFrame, NuSeries},
    Signature, SyntaxShape,
};

use nu_source::Tagged;

use super::utils::parse_polars_error;
pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "dataframe column"
    }

    fn usage(&self) -> &str {
        "[DataFrame] Returns the selected column as Series"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe column").required("column", SyntaxShape::String, "column name")
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Returns the selected column as series",
            example: "[[a b]; [1 2] [3 4]] | dataframe to-df | dataframe column a",
            result: None,
        }]
    }
}

fn command(mut args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();
    let column: Tagged<String> = args.req(0)?;

    let df = NuDataFrame::try_from_stream(&mut args.input, &tag.span)?;

    let res = df
        .as_ref()
        .column(column.item.as_ref())
        .map_err(|e| parse_polars_error::<&str>(&e, &column.tag.span, None))?;

    Ok(OutputStream::one(NuSeries::series_to_value(
        res.clone(),
        tag,
    )))
}
