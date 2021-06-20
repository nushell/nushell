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
        "dataframe with-column"
    }

    fn usage(&self) -> &str {
        "[DataFrame] Adds a series to the dataframe"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe with-column")
            .required("series", SyntaxShape::Any, "series to be added")
            .required_named("name", SyntaxShape::String, "column name", Some('n'))
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Adds a series to the dataframe",
            example:
                "[[a b]; [1 2] [3 4]] | dataframe to-df | dataframe with-column ([5 6] | dataframe to-series) --name c",
            result: None,
        }]
    }
}

fn command(mut args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();
    let value: Value = args.req(0)?;
    let name: Tagged<String> = args.req_named("name")?;

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

    df.as_mut()
        .with_column(series)
        .map_err(|e| parse_polars_error::<&str>(&e, &tag.span, None))?;

    Ok(OutputStream::one(df.into_value(tag)))
}
