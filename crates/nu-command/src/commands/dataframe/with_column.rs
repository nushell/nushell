use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    dataframe::{NuDataFrame, PolarsData},
    Signature, SyntaxShape, UntaggedValue, Value,
};
use nu_source::Tagged;

use super::utils::parse_polars_error;
pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "pls with-column"
    }

    fn usage(&self) -> &str {
        "Adds a series to the dataframe"
    }

    fn signature(&self) -> Signature {
        Signature::build("pls with-column")
            .required("series", SyntaxShape::Any, "series to be added")
            .required("as", SyntaxShape::String, "the word 'as'")
            .required("name", SyntaxShape::String, "column name")
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Adds a series to the dataframe",
            example:
                "[[a b]; [1 2] [3 4]] | pls to-df | pls with-column ([5 6] | pls to-series) as c",
            result: None,
        }]
    }
}

fn command(mut args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();
    let value: Value = args.req(0)?;
    let name: Tagged<String> = args.req(2)?;

    let mut series = match value.value {
        UntaggedValue::DataFrame(PolarsData::Series(series)) => Ok(series),
        _ => Err(ShellError::labeled_error(
            "Incorrect type",
            "can only add a series to a dataframe",
            value.tag.span,
        )),
    }?;

    let series = series.as_mut().rename(name.item.as_ref()).clone();

    let mut df = NuDataFrame::try_from_stream(&mut args.input, &tag.span)?;

    let res = df
        .as_mut()
        .with_column(series)
        .map_err(|e| parse_polars_error::<&str>(&e, &tag.span, None))?;

    Ok(OutputStream::one(NuDataFrame::dataframe_to_value(
        res.clone(),
        tag,
    )))
}
