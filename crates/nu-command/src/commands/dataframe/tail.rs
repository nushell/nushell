use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    dataframe::{NuDataFrame, PolarsData},
    Signature, SyntaxShape, UntaggedValue, Value,
};

use nu_source::Tagged;
pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "pls tail"
    }

    fn usage(&self) -> &str {
        "Creates new dataframe with tail rows"
    }

    fn signature(&self) -> Signature {
        Signature::build("pls select").optional(
            "n_rows",
            SyntaxShape::Number,
            "Number of rows for tail",
        )
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Create new dataframe with tail rows",
            example: "echo [[a b]; [1 2] [3 4]] | pls convert | pls tail",
            result: None,
        }]
    }
}

fn command(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();
    let mut args = args.evaluate_once()?;
    let rows: Option<Tagged<usize>> = args.opt(0)?;

    let rows = match rows {
        Some(val) => val.item,
        None => 5,
    };

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
                let res = df.tail(Some(rows));

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
