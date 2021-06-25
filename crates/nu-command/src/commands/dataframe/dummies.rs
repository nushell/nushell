use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    dataframe::{NuDataFrame, PolarsData},
    Signature, UntaggedValue,
};

use super::utils::parse_polars_error;

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "dataframe to-dummies"
    }

    fn usage(&self) -> &str {
        "[DataFrame] Creates a new dataframe with dummy variables"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe to-dummies")
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Create new dataframe with dummy variables from a dataframe",
                example: "[[a b]; [1 2] [3 4]] | dataframe to-df | dataframe to-dummies",
                result: None,
            },
            Example {
                description: "Create new dataframe with dummy variables from a series",
                example: "[1 2 2 3 3] | dataframe to-series | dataframe to-dummies",
                result: None,
            },
        ]
    }
}

fn command(mut args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();

    let value = args.input.next().ok_or_else(|| {
        ShellError::labeled_error("Empty stream", "No value found in stream", &tag.span)
    })?;

    match value.value {
        UntaggedValue::DataFrame(PolarsData::EagerDataFrame(df)) => {
            let res = df.as_ref().to_dummies().map_err(|e| {
                parse_polars_error(
                    &e,
                    &tag.span,
                    Some("The only allowed column types for dummies are String or Int"),
                )
            })?;

            Ok(OutputStream::one(NuDataFrame::dataframe_to_value(res, tag)))
        }
        UntaggedValue::DataFrame(PolarsData::Series(series)) => {
            let res = series.as_ref().to_dummies().map_err(|e| {
                parse_polars_error(
                    &e,
                    &tag.span,
                    Some("The only allowed column types for dummies are String or Int"),
                )
            })?;

            Ok(OutputStream::one(NuDataFrame::dataframe_to_value(res, tag)))
        }
        _ => Err(ShellError::labeled_error(
            "Incorrect type",
            "dummies cannot be done with this value",
            &value.tag.span,
        )),
    }
}
