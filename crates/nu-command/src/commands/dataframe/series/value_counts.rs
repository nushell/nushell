use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    dataframe::{NuDataFrame, NuSeries},
    Signature,
};

use crate::commands::dataframe::utils::parse_polars_error;

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "dataframe value-counts"
    }

    fn usage(&self) -> &str {
        "[Series] Returns a dataframe with the counts for unique values in series"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe value-counts")
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Calculates value counts",
            example: "[5 5 6 6] | dataframe to-series | dataframe value-counts",
            result: None,
        }]
    }
}

fn command(mut args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();

    let series = NuSeries::try_from_stream(&mut args.input, &tag.span)?;

    let df = series
        .as_ref()
        .value_counts()
        .map_err(|e| parse_polars_error::<&str>(&e, &tag.span, None))?;

    Ok(OutputStream::one(NuDataFrame::dataframe_to_value(df, tag)))
}
