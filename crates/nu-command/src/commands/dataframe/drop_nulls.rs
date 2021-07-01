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
        "dataframe drop-nulls"
    }

    fn usage(&self) -> &str {
        "[DataFrame, Series] Drops null values in dataframe"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe drop-nulls").optional(
            "subset",
            SyntaxShape::Table,
            "subset of columns to drop duplicates",
        )
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "drop null values in dataframe",
                example: r#"let df = ([[a b]; [1 2] [3 0] [1 2]] | dataframe to-df);
    let res = ($df.b / $df.b);
    let df = ($df | dataframe with-column $res --name res);
    $df | dataframe drop-nulls
"#,
                result: None,
            },
            Example {
                description: "drop null values in dataframe",
                example: r#"let s = ([1 2 0 0 3 4] | dataframe to-series);
    ($s / $s) | dataframe drop-nulls"#,
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
            // Extracting the selection columns of the columns to perform the aggregation
            let columns: Option<Vec<Value>> = args.opt(0)?;
            let (subset, col_span) = match columns {
                Some(cols) => {
                    let (agg_string, col_span) = convert_columns(&cols, &tag)?;
                    (Some(agg_string), col_span)
                }
                None => (None, Span::unknown()),
            };

            let subset_slice = subset.as_ref().map(|cols| &cols[..]);

            let res = df
                .as_ref()
                .drop_nulls(subset_slice)
                .map_err(|e| parse_polars_error::<&str>(&e, &col_span, None))?;

            Ok(OutputStream::one(NuDataFrame::dataframe_to_value(res, tag)))
        }
        UntaggedValue::DataFrame(PolarsData::Series(series)) => {
            let res = series.as_ref().drop_nulls();
            Ok(OutputStream::one(NuSeries::series_to_value(res, tag)))
        }
        _ => Err(ShellError::labeled_error(
            "Incorrect type",
            "drop nulls cannot be done with this value",
            &value.tag.span,
        )),
    }
}
