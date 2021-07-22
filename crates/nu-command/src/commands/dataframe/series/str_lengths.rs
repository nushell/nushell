use crate::{commands::dataframe::utils::parse_polars_error, prelude::*};
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{dataframe::NuDataFrame, Signature};
use polars::prelude::IntoSeries;

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "dataframe str-lengths"
    }

    fn usage(&self) -> &str {
        "[Series] Get lengths of all strings"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe str-lengths")
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Returns string lengths",
            example: "[a ab abc] | dataframe to-df | dataframe str-lengths",
            result: None,
        }]
    }
}

fn command(mut args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();

    let (df, df_tag) = NuDataFrame::try_from_stream(&mut args.input, &tag.span)?;

    let series = df.as_series(&df_tag.span)?;
    let chunked = series.utf8().map_err(|e| {
        parse_polars_error::<&str>(
            &e,
            &df_tag.span,
            Some("The str-lengths command can only be used with string columns"),
        )
    })?;

    let res = chunked.as_ref().str_lengths();

    let df = NuDataFrame::try_from_series(vec![res.into_series()], &tag.span)?;
    Ok(OutputStream::one(df.into_value(df_tag)))
}
