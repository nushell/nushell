use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{dataframe::NuSeries, Signature};
use polars::prelude::IntoSeries;

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "dataframe is-null"
    }

    fn usage(&self) -> &str {
        "[Series] Creates mask where value is null"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe is-null")
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Create mask where values are null",
            example: r#"let s = ([5 6 0 8] | dataframe to-series);
    let res = ($s / $s);
    $res | dataframe is-null"#,
            result: None,
        }]
    }
}

fn command(mut args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();

    let series = NuSeries::try_from_stream(&mut args.input, &tag.span)?;

    let res = series.as_ref().is_null();

    Ok(OutputStream::one(NuSeries::series_to_value(
        res.into_series(),
        tag,
    )))
}
