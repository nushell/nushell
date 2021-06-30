use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    dataframe::{NuDataFrame, NuSeries, PolarsData},
    Signature, SyntaxShape, UntaggedValue, Value,
};

use super::utils::{convert_columns, parse_polars_error};
pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "dataframe sort"
    }

    fn usage(&self) -> &str {
        "[DataFrame, Series] Creates new sorted dataframe or series"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe sort")
            .switch("reverse", "invert sort", Some('r'))
            .rest(SyntaxShape::Any, "column names to sort dataframe")
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Create new sorted dataframe",
                example: "[[a b]; [3 4] [1 2]] | dataframe to-df | dataframe sort a",
                result: None,
            },
            Example {
                description: "Create new sorted series",
                example: "[3 4 1 2] | dataframe to-series | dataframe sort",
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

    let reverse = args.has_flag("reverse");

    match value.value {
        UntaggedValue::DataFrame(PolarsData::EagerDataFrame(df)) => {
            let columns: Vec<Value> = args.rest(0)?;

            if !columns.is_empty() {
                let (col_string, col_span) = convert_columns(&columns, &tag)?;

                let res = df
                    .as_ref()
                    .sort(&col_string, reverse)
                    .map_err(|e| parse_polars_error::<&str>(&e, &col_span, None))?;

                Ok(OutputStream::one(NuDataFrame::dataframe_to_value(res, tag)))
            } else {
                Err(ShellError::labeled_error(
                    "Missing columns",
                    "missing column name to perform sort",
                    &tag.span,
                ))
            }
        }
        UntaggedValue::DataFrame(PolarsData::Series(series)) => {
            let res = series.as_ref().sort(reverse);
            Ok(OutputStream::one(NuSeries::series_to_value(res, tag)))
        }
        _ => Err(ShellError::labeled_error(
            "Incorrect type",
            "sort cannot be done with this value",
            &value.tag.span,
        )),
    }
}
