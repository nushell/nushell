use crate::{commands::dataframe::utils::parse_polars_error, prelude::*};
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{dataframe::NuDataFrame, Signature};
use polars::prelude::IntoSeries;

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "dataframe arg-true"
    }

    fn usage(&self) -> &str {
        "[Series] Returns indexes where values are true"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe arg-true")
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Returns indexes where values are true",
            example: "[$false $true $false] | dataframe to-df | dataframe arg-true",
            result: None,
        }]
    }
}

fn command(mut args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();

    let (df, df_tag) = NuDataFrame::try_from_stream(&mut args.input, &tag.span)?;

    let series = df.as_series(&df_tag.span)?;
    let bool = series.bool().map_err(|e| {
        parse_polars_error::<&str>(
            &e,
            &tag.span,
            Some("arg-true only works with series of type bool"),
        )
    })?;

    let mut res = bool.arg_true().into_series();
    res.rename("int");

    let df = NuDataFrame::try_from_series(vec![res], &tag.span)?;
    Ok(OutputStream::one(df.into_value(df_tag)))
}
