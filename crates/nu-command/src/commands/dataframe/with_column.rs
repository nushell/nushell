use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    dataframe::{NuDataFrame, PolarsData},
    Signature, SyntaxShape, UntaggedValue, Value,
};

use super::utils::parse_polars_error;
pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "pls with_column"
    }

    fn usage(&self) -> &str {
        "Adds a series to the dataframe"
    }

    fn signature(&self) -> Signature {
        Signature::build("pls with_column").required(
            "series",
            SyntaxShape::Any,
            "series to be added",
        )
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Adds a series to the dataframe",
            example: "[[a b]; [1 2] [3 4]] | pls to_df | pls with_column ([5 6] | pls to_series)",
            result: None,
        }]
    }
}

fn command(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();
    let mut args = args.evaluate_once()?;
    let value: Value = args.req(0)?;

    let series = match value.value {
        UntaggedValue::DataFrame(PolarsData::Series(series)) => Ok(series),
        _ => Err(ShellError::labeled_error(
            "Incorrect type",
            "can only add a series to a dataframe",
            value.tag.span,
        )),
    }?;

    let mut df = NuDataFrame::try_from_stream(&mut args.input, &tag.span)?;

    let res = df
        .as_mut()
        .with_column(series.series())
        .map_err(|e| parse_polars_error::<&str>(&e, &tag.span, None))?;

    Ok(OutputStream::one(NuDataFrame::dataframe_to_value(
        res.clone(),
        tag,
    )))
}
