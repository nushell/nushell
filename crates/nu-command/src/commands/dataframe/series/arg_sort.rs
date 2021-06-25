use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{dataframe::NuSeries, Signature};
use polars::prelude::IntoSeries;

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "dataframe arg-sort"
    }

    fn usage(&self) -> &str {
        "[Series] Returns indexes for a sorted series"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe arg-sort").switch("reverse", "reverse order", Some('r'))
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Returns indexes for a sorted series",
            example: "[1 2 2 3 3] | dataframe to-series | dataframe arg-sort",
            result: None,
        }]
    }
}

fn command(mut args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();
    let reverse = args.has_flag("reverse");

    let series = NuSeries::try_from_stream(&mut args.input, &tag.span)?;

    let res = series.as_ref().argsort(reverse);

    Ok(OutputStream::one(NuSeries::series_to_value(
        res.into_series(),
        tag,
    )))
}
