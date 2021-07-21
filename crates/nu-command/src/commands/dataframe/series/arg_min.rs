use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{dataframe::NuDataFrame, Signature};

use polars::prelude::{IntoSeries, NewChunkedArray, UInt32Chunked};

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "dataframe arg-min"
    }

    fn usage(&self) -> &str {
        "[Series] Return index for min value in series"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe arg-min")
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Returns index for min value",
            example: "[1 3 2] | dataframe to-df | dataframe arg-min",
            result: None,
        }]
    }
}

fn command(mut args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();

    let (df, df_tag) = NuDataFrame::try_from_stream(&mut args.input, &tag.span)?;

    let res = df.as_series(&df_tag.span)?.arg_min();

    let chunked = match res {
        Some(index) => UInt32Chunked::new_from_slice("arg_min", &[index as u32]),
        None => UInt32Chunked::new_from_slice("arg_min", &[]),
    };

    let res = chunked.into_series();
    let df = NuDataFrame::try_from_series(vec![res], &tag.span)?;

    Ok(OutputStream::one(df.into_value(df_tag)))
}
