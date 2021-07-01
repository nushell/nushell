use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{dataframe::NuDataFrame, Signature, SyntaxShape, Value};

use super::utils::{convert_columns, parse_polars_error};
pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "dataframe get"
    }

    fn usage(&self) -> &str {
        "[DataFrame] Creates dataframe with the selected columns"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe get").rest(SyntaxShape::Any, "column names to sort dataframe")
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Creates dataframe with selected columns",
            example: "[[a b]; [1 2] [3 4]] | dataframe to-df | dataframe get a",
            result: None,
        }]
    }
}

fn command(mut args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();
    let columns: Vec<Value> = args.rest(0)?;

    let (col_string, col_span) = convert_columns(&columns, &tag)?;

    let df = NuDataFrame::try_from_stream(&mut args.input, &tag.span)?;

    let res = df
        .as_ref()
        .select(&col_string)
        .map_err(|e| parse_polars_error::<&str>(&e, &col_span, None))?;

    Ok(OutputStream::one(NuDataFrame::dataframe_to_value(res, tag)))
}
