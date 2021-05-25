use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{dataframe::NuDataFrame, Signature, SyntaxShape, UntaggedValue, Value};

use super::utils::convert_columns;

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "dataframe drop"
    }

    fn usage(&self) -> &str {
        "Creates a new dataframe by dropping the selected columns"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe drop").required(
            "columns",
            SyntaxShape::Table,
            "column names to be dropped",
        )
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        drop(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "drop column a",
            example: "echo [[a b]; [1 2] [3 4]] | dataframe | dataframe drop [a]",
            result: None,
        }]
    }
}

fn drop(args: CommandArgs) -> Result<OutputStream, ShellError> {
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
            if let UntaggedValue::DataFrame(NuDataFrame {
                dataframe: Some(ref df),
                ..
            }) = value.value
            {
                let new_df = match col_string.iter().next() {
                    Some(col) => df.drop(col).map_err(|e| {
                        ShellError::labeled_error("Join error", format!("{}", e), &col_span)
                    }),
                    None => Err(ShellError::labeled_error(
                        "Empty names list",
                        "No column names where found",
                        &col_span,
                    )),
                }?;

                let res = col_string.iter().skip(1).try_fold(new_df, |new_df, col| {
                    new_df.drop(col).map_err(|e| {
                        ShellError::labeled_error("Drop error", format!("{}", e), &col_span)
                    })
                })?;

                let value = Value {
                    value: UntaggedValue::DataFrame(NuDataFrame::new(res)),
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
