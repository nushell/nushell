use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{dataframe::NuDataFrame, Signature};

use polars::prelude::{IntoSeries, NewChunkedArray, UInt32Chunked};

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "dataframe arg-max"
    }

    fn usage(&self) -> &str {
        "[Series] Return index for max value in series"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe arg-max")
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Returns index for max value",
            example: "[1 3 2] | dataframe to-df | dataframe arg-max",
            result: None,
        }]
    }
}

fn command(mut args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();

    let (df, df_tag) = NuDataFrame::try_from_stream(&mut args.input, &tag.span)?;
    let series = df.as_series(&df_tag.span)?;

    let res = series.arg_max();

    let chunked = match res {
        Some(index) => UInt32Chunked::new_from_slice("arg_max", &[index as u32]),
        None => UInt32Chunked::new_from_slice("arg_max", &[]),
    };

    let res = chunked.into_series();
    let df = NuDataFrame::try_from_series(vec![res], &tag.span)?;

    Ok(OutputStream::one(df.into_value(df_tag)))
}
