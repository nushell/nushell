use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{dataframe::NuDataFrame, Signature, SyntaxShape, Value};

use super::utils::{convert_columns, parse_polars_error};

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "pls drop_nulls"
    }

    fn usage(&self) -> &str {
        "Drops null values in dataframe"
    }

    fn signature(&self) -> Signature {
        Signature::build("pls drop_nulls").optional(
            "subset",
            SyntaxShape::Table,
            "subset of columns to drop duplicates",
        )
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "drop null values duplicates",
            example: "[[a b]; [1 2] [3 4] [1 2]] | pls convert | pls drop_nulls",
            result: None,
        }]
    }
}

fn command(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();
    let mut args = args.evaluate_once()?;

    // Extracting the selection columns of the columns to perform the aggregation
    let columns: Option<Vec<Value>> = args.opt(0)?;
    let (subset, col_span) = match columns {
        Some(cols) => {
            let (agg_string, col_span) = convert_columns(&cols, &tag)?;
            (Some(agg_string), col_span)
        }
        None => (None, Span::unknown()),
    };

    let df = NuDataFrame::try_from_stream(&mut args.input, &tag.span)?;

    let subset_slice = subset.as_ref().map(|cols| &cols[..]);

    let res = df
        .as_ref()
        .drop_nulls(subset_slice)
        .map_err(|e| parse_polars_error::<&str>(&e, &col_span, None))?;

    Ok(OutputStream::one(NuDataFrame::dataframe_to_value(res, tag)))
}
