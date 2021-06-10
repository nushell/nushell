use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{dataframe::NuDataFrame, Signature};

use super::utils::parse_polars_error;

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "pls to-dummies"
    }

    fn usage(&self) -> &str {
        "Creates a new dataframe with dummy variables"
    }

    fn signature(&self) -> Signature {
        Signature::build("pls to-dummies")
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Create new dataframe with dummy variables",
            example: "[[a b]; [1 2] [3 4]] | pls to-df | pls to-dummies",
            result: None,
        }]
    }
}

fn command(mut args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();

    let df = NuDataFrame::try_from_stream(&mut args.input, &tag.span)?;
    let res = df.as_ref().to_dummies().map_err(|e| {
        parse_polars_error(
            &e,
            &tag.span,
            Some("The only allowed column types for dummies are String or Int"),
        )
    })?;

    Ok(OutputStream::one(NuDataFrame::dataframe_to_value(res, tag)))
}
