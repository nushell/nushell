use crate::{commands::dataframe::utils::parse_polars_error, prelude::*};
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{dataframe::NuSeries, Signature};
use polars::prelude::IntoSeries;
use std::ops::Not;

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "dataframe not"
    }

    fn usage(&self) -> &str {
        "[Series] Inverts boolean mask"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe not")
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Inverts boolean mask",
            example: "[$true $false $true] | dataframe to-series | dataframe not",
            result: None,
        }]
    }
}

fn command(mut args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();

    let series = NuSeries::try_from_stream(&mut args.input, &tag.span)?;

    let bool = series.as_ref().bool().map_err(|e| {
        parse_polars_error::<&str>(
            &e,
            &tag.span,
            Some("not only works with series of type bool"),
        )
    })?;

    let res = bool.not();

    Ok(OutputStream::one(NuSeries::series_to_value(
        res.into_series(),
        tag,
    )))
}
