use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{dataframe::NuDataFrame, Signature, SyntaxShape, Value};

use super::utils::{convert_columns, parse_polars_error};
pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "pls sort"
    }

    fn usage(&self) -> &str {
        "Creates new sorted dataframe"
    }

    fn signature(&self) -> Signature {
        Signature::build("pls sort")
            .required(
                "columns",
                SyntaxShape::Table,
                "column names to sort dataframe",
            )
            .switch("reverse", "invert sort", Some('r'))
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Create new sorted dataframe",
            example: "[[a b]; [3 4] [1 2]] | pls to-df | pls sort [a]",
            result: None,
        }]
    }
}

fn command(mut args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();
    let columns: Vec<Value> = args.req(0)?;
    let reverse = args.has_flag("reverse");

    let (col_string, col_span) = convert_columns(&columns, &tag)?;

    let df = NuDataFrame::try_from_stream(&mut args.input, &tag.span)?;

    let res = df
        .as_ref()
        .sort(&col_string, reverse)
        .map_err(|e| parse_polars_error::<&str>(&e, &col_span, None))?;

    Ok(OutputStream::one(NuDataFrame::dataframe_to_value(res, tag)))
}
