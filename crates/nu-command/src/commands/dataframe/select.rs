use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    dataframe::{NuDataFrame, PolarsData},
    Signature, SyntaxShape, UntaggedValue, Value,
};

use super::utils::convert_columns;

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "pls select"
    }

    fn usage(&self) -> &str {
        "Creates a new dataframe with the selected columns"
    }

    fn signature(&self) -> Signature {
        Signature::build("pls select").required(
            "columns",
            SyntaxShape::Table,
            "selected column names",
        )
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        select(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Create new dataframe with column a",
            example: "echo [[a b]; [1 2] [3 4]] | pls convert | pls select [a]",
            result: None,
        }]
    }
}

fn select(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();
    let mut args = args.evaluate_once()?;

    let columns: Vec<Value> = args.req(0)?;

    let (col_string, col_span) = convert_columns(&columns, &tag)?;

    match args.input.next() {
        None => Err(ShellError::labeled_error(
            "No input received",
            "missing dataframe input from stream",
            &tag,
        )),
        Some(value) => {
            if let UntaggedValue::DataFrame(PolarsData::EagerDataFrame(NuDataFrame {
                dataframe: Some(ref df),
                ..
            })) = value.value
            {
                let res = df.select(&col_string).map_err(|e| {
                    ShellError::labeled_error("Drop error", format!("{}", e), &col_span)
                })?;

                let value = Value {
                    value: UntaggedValue::DataFrame(PolarsData::EagerDataFrame(NuDataFrame::new(
                        res,
                    ))),
                    tag: tag.clone(),
                };

                Ok(OutputStream::one(value))
            } else {
                Err(ShellError::labeled_error(
                    "No dataframe in stream",
                    "no dataframe found in input stream",
                    &tag,
                ))
            }
        }
    }
}
