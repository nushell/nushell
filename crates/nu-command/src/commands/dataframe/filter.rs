use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{dataframe::NuDataFrame, Signature, SyntaxShape, UntaggedValue, Value};

use super::utils::parse_polars_error;
pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "dataframe filter-with"
    }

    fn usage(&self) -> &str {
        "[DataFrame] Filters dataframe using a mask as reference"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe filter-with").required(
            "mask",
            SyntaxShape::Any,
            "boolean mask used to filter data",
        )
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Filter dataframe using a bool mask",
                example: r#"let mask = ([$true $false] | dataframe to-df);
    [[a b]; [1 2] [3 4]] | dataframe to-df | dataframe filter-with $mask"#,
                result: None,
            },
            Example {
                description: "Filter dataframe by creating a mask from operation",
                example: r#"let mask = (([5 6] | dataframe to-df) > 5);
    [[a b]; [1 2] [3 4]] | dataframe to-df | dataframe filter-with $mask"#,
                result: None,
            },
        ]
    }
}

fn command(mut args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();
    let value: Value = args.req(0)?;

    let series_span = value.tag.span;
    let df = match value.value {
        UntaggedValue::DataFrame(df) => Ok(df),
        _ => Err(ShellError::labeled_error(
            "Incorrect type",
            "can only add a series to a dataframe",
            value.tag.span,
        )),
    }?;
    let series = df.as_series(&series_span)?;
    let casted = series.bool().map_err(|e| {
        parse_polars_error(
            &e,
            &&series_span,
            Some("Perhaps you want to use a series with booleans as mask"),
        )
    })?;

    let (df, df_tag) = NuDataFrame::try_from_stream(&mut args.input, &tag.span)?;

    let res = df
        .as_ref()
        .filter(&casted)
        .map_err(|e| parse_polars_error::<&str>(&e, &df_tag.span, None))?;

    Ok(OutputStream::one(NuDataFrame::dataframe_to_value(res, tag)))
}
